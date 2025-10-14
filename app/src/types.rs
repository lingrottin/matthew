use axum::response::IntoResponse;
use serde::Deserialize;
use serde::Serialize;

pub enum ErrorType {
    Matthew((u16, String)),
    Other(anyhow::Error),
}
impl Into<ErrorType> for anyhow::Error {
    fn into(self) -> ErrorType {
        ErrorType::Other(self)
    }
}
pub struct ApiError {
    err: ErrorType,
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self.err {
            ErrorType::Other(e) => {
                return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                    .into_response();
            }
            ErrorType::Matthew(e) => {
                return (axum::http::StatusCode::from_u16(e.0).unwrap(), e.1).into_response();
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, ApiError>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Repo {
    pub owner: String,
    pub repo: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct InvokeApiInput {
    pub repo: String,
    pub user: String,
    pub callback: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct Output {
    pub success: bool,
}
impl IntoResponse for Output {
    fn into_response(self) -> axum::response::Response {
        let status = if self.success {
            axum::http::StatusCode::OK
        } else {
            axum::http::StatusCode::UNAUTHORIZED
        };
        (status, serde_json::to_string(&self).unwrap()).into_response()
    }
}

#[derive(Serialize, Clone, Debug)]
pub enum ItemStatus {
    Done,
    Error,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct ItemData {
    pub lorc: u64,
    pub counts: matthew::Counts,
}

#[derive(Serialize, Clone, Debug)]
pub struct ItemCallback {
    pub repo: String,
    pub status: ItemStatus,
    pub data: Option<ItemData>,
    pub error: Option<String>,
}
