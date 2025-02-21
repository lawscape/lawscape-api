use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ApiServerError {
    #[error("runtime error at tokio")]
    TokioRuntime,
    #[error("error at tracing_subscriber")]
    LoggerConfig,
    #[error("axum error")]
    AxumError,
    #[error("meilisearch error")]
    MeilisearchError,
    #[error("search error")]
    SearchError,
}

impl IntoResponse for ApiServerError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{self}")).into_response()
    }
}
