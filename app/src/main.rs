use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Semaphore;
use tracing::{info, warn};

use crate::types::{Output, Repo};
mod service;
mod types;

const TOKIO_WORKER_STACK_SIZE: usize = 32 * 1024 * 1024;

fn env_or_config(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

async fn maybe_read_config() -> Option<toml::Value> {
    tokio::fs::read_to_string("Config.toml")
        .await
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
}

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(TOKIO_WORKER_STACK_SIZE)
        .build()
        .expect("failed to build tokio runtime");

    runtime.block_on(async_main());
}

async fn async_main() {
    tracing_subscriber::fmt::init();

    let config = maybe_read_config().await;

    // Helper: get string from env > Config.toml
    let cfg_str = |key: &str, env: &str| -> Option<String> {
        env_or_config(env)
            .or_else(|| config.as_ref().and_then(|c| c.get(key).and_then(|v| v.as_str()).map(String::from)))
    };

    // Helper: get integer from env > Config.toml > default
    let cfg_int = |key: &str, env: &str, default: u64| -> u64 {
        env_or_config(env)
            .and_then(|v| v.parse().ok())
            .or_else(|| config.as_ref().and_then(|c| c.get(key).and_then(|v| v.as_integer()).map(|v| v as u64)))
            .unwrap_or(default)
    };

    let port: u16 = cfg_int("port", "MATTHEW_PORT", 3000) as u16;
    let token = cfg_str("token", "MATTHEW_TOKEN").unwrap_or_default();
    let callback_secret = cfg_str("callback_secret", "MATTHEW_CALLBACK_SECRET").unwrap_or_default();
    let max_repo_size_kb = cfg_int("max_repo_size_kb", "MATTHEW_MAX_REPO_SIZE_KB", 4 * 1024 * 1024);

    info!(port, token_configured = !token.is_empty(), callback_secret_configured = !callback_secret.is_empty(), max_repo_size_kb, "configuration loaded");

    let state = AppState {
        sem: Arc::new(Semaphore::new(4)), // limit to 4 concurrent tasks
        data_dir: PathBuf::from("./data"),
        client: Arc::new(reqwest::Client::new()),
        token,
        callback_secret,
        max_repo_size_kb,
    };
    let app: Router = Router::new()
        .route("/api/count", post(handle_request))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    info!(%addr, "starting server");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Clone)]
pub struct AppState {
    sem: Arc<Semaphore>,
    data_dir: PathBuf,
    client: Arc<reqwest::Client>,
    token: String,
    callback_secret: String,
    max_repo_size_kb: u64,
}

async fn handle_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(repo): Json<types::InvokeApiInput>,
) -> types::Result<types::Output> {
    info!(user = %repo.user, repo = %repo.repo, "received count request");
    if !state.token.is_empty()
        && headers.get("Authorization").and_then(|v| v.to_str().ok())
            != Some(&format!("Bearer {}", state.token))
    {
        warn!(user = %repo.user, repo = %repo.repo, "unauthorized request rejected");
        return Ok(types::Output { success: false });
    }
    let state_clone = state.clone();
    tokio::spawn(async move {
        let state = state_clone;
        let repo = repo;
        let closure = async || -> anyhow::Result<types::ItemData> {
            let _permit = state.sem.acquire().await?;
            let repo = repo.clone();
            service::count(
                state.data_dir.clone(),
                Repo {
                    owner: repo.user,
                    repo: repo.repo,
                },
                state.client.clone(),
                repo.token.clone(),
                state.max_repo_size_kb,
            )
            .await
        };
        let res = closure().await;
        let callback = match res {
            Ok(data) => types::ItemCallback {
                repo: repo.repo,
                status: types::ItemStatus::Done,
                data: Some(data),
                error: None,
            },
            Err(e) => types::ItemCallback {
                repo: repo.repo,
                status: types::ItemStatus::Error,
                data: None,
                error: Some(e.to_string()),
            },
        };
        let body_json = serde_json::to_string(&callback)?;
        let signature = hmac_sign(&state.callback_secret, &body_json);
        state
            .client
            .post(&repo.callback)
            .header("User-Agent", "Matthew")
            .header("Content-Type", "application/json")
            .header("X-Signature-256", format!("sha256={}", signature))
            .body(body_json)
            .send()
            .await?;
        anyhow::Ok(())
    });
    Ok(Output { success: true })
}

fn hmac_sign(secret: &str, body: &str) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(body.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}
