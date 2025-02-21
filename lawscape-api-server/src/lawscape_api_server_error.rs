use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ApiServerError {
  #[error("runtime error at tokio")]
  TokioRuntime,
  #[error("error at tracing_subscriber")]
  LoggerConfig,
}
