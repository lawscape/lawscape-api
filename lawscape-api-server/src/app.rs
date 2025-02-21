use axum::{
  http::Method,
  routing::{delete, get, post},
  Router,
};
use std::net::SocketAddr;
use crate::lawscape_api_server_error::ApiServerError;

/// ログを出力するための設定など
async fn init_logger() -> Result<(), Box<dyn std::error::Error>> {
  let subscriber = tracing_subscriber::fmt()
      .with_max_level(tracing::Level::INFO)
      .finish();
  tracing::subscriber::set_global_default(subscriber).map_err(|_| ApiServerError::LoggerConfig)?;
  Ok(())
}

pub async fn app(bind: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
  init_logger().await?;
  // TODO
  Ok(())
}