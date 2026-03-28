use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

use crate::types::{Output, Repo};
mod service;
mod types;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config_toml = std::fs::read_to_string("Config.toml").expect("Failed to read Config.toml");
    let config: toml::Value = toml::from_str(&config_toml).expect("Failed to parse Config.toml");
    let mut port: i64 = 3000;
    if let Some(port_i) = config.get("port").and_then(|v| v.as_integer()) {
        port = port_i;
    }
    let token = if let Some(token) = config.get("token").and_then(|v| v.as_str()) {
        token.to_string()
    } else {
        panic!("token not found in Config.toml");
    };
    let state = AppState {
        sem: Arc::new(Semaphore::new(4)), // limit to 4 concurrent tasks
        data_dir: PathBuf::from("./data"),
        client: Arc::new(reqwest::Client::new()),
        token,
    };
    let app: Router = Router::new()
        .route("/api/count", post(handle_request))
        .with_state(state);

    info!(port = port, "starting server");
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Clone)]
pub struct AppState {
    sem: Arc<Semaphore>,
    data_dir: PathBuf,
    client: Arc<reqwest::Client>,
    token: String,
}

async fn handle_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(repo): Json<types::InvokeApiInput>,
) -> types::Result<types::Output> {
    info!(owner = %repo.user, repo = %repo.repo, callback = %repo.callback, "received count request");
    if headers.get("Authorization").and_then(|v| v.to_str().ok())
        != Some(&format!("Bearer {}", state.token))
    {
        warn!(owner = %repo.user, repo = %repo.repo, "authorization failed");
        return Ok(types::Output { success: false });
    }
    let state_clone = state.clone();
    tokio::spawn(async move {
        let state = state_clone;
        let repo = repo;
        info!(owner = %repo.user, repo = %repo.repo, "background count task started");
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
            )
            .await
        };
        let res = closure().await;
        let repo_name = repo.repo.clone();
        let callback = match res {
            Ok(data) => {
                info!(owner = %repo.user, repo = %repo_name, "background count task completed");
                types::ItemCallback {
                    repo: repo_name.clone(),
                    status: types::ItemStatus::Done,
                    data: Some(data),
                    error: None,
                }
            }
            Err(e) => {
                error!(owner = %repo.user, repo = %repo_name, error = %e, "background count task failed");
                types::ItemCallback {
                    repo: repo_name.clone(),
                    status: types::ItemStatus::Error,
                    data: None,
                    error: Some(e.to_string()),
                }
            }
        };
        let send_result = state
            .client
            .post(&repo.callback)
            .header("User-Agent", "Matthew")
            .json(&callback)
            .send()
            .await
            .map(|_| ());

        if let Err(e) = send_result {
            error!(owner = %repo.user, repo = %repo_name, callback = %repo.callback, error = %e, "failed to send callback");
        } else {
            info!(owner = %repo.user, repo = %repo_name, callback = %repo.callback, "callback sent");
        }
    });
    Ok(Output { success: true })
}
