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

    let config_toml = std::fs::read_to_string("Config.toml").expect("Failed to read Config.toml");
    let config: toml::Value = toml::from_str(&config_toml).expect("Failed to parse Config.toml");
    let mut port: i64 = 3000;
    if let Some(port_i) = config.get("port").and_then(|v| v.as_integer()) {
        port = port_i;
    }
    info!(port, "loaded configuration");
    let token = if let Some(token) = config.get("token").and_then(|v| v.as_str()) {
        token.to_string()
    } else {
        panic!("token not found in Config.toml");
    };
    let callback_secret = if let Some(s) = config.get("callback_secret").and_then(|v| v.as_str()) {
        s.to_string()
    } else {
        panic!("callback_secret not found in Config.toml");
    };
    let state = AppState {
        sem: Arc::new(Semaphore::new(4)), // limit to 4 concurrent tasks
        data_dir: PathBuf::from("./data"),
        client: Arc::new(reqwest::Client::new()),
        token,
        callback_secret,
    };
    let app: Router = Router::new()
        .route("/api/count", post(handle_request))
        .with_state(state);

    let addr = format!("127.0.0.1:{}", port);
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
}

async fn handle_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(repo): Json<types::InvokeApiInput>,
) -> types::Result<types::Output> {
    info!(user = %repo.user, repo = %repo.repo, "received count request");
    if headers.get("Authorization").and_then(|v| v.to_str().ok())
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
