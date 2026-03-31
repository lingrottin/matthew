use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Semaphore;

use crate::types::{Output, Repo};
mod service;
mod types;

#[tokio::main]
async fn main() {
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
    if headers.get("Authorization").and_then(|v| v.to_str().ok())
        != Some(&format!("Bearer {}", state.token))
    {
        return Ok(types::Output { success: false });
    }
    let state_clone = state.clone();
    tokio::spawn(async move {
        let state = state_clone;
        state.sem.acquire().await.unwrap().forget();
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
        state
            .client
            .post(&repo.callback)
            .header("User-Agent", "Matthew")
            .json(&callback)
            .send()
            .await
            .unwrap();
    });
    Ok(Output { success: true })
}
