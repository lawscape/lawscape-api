use clap::Parser;
use tokio::runtime;
use std::net::SocketAddr;


mod lawscape_api_server_error;
use lawscape_api_server_error::ApiServerError;

mod app;

#[derive(Debug, Parser)]
struct AppArg {
  #[arg(long, env)]
  pub meilisearch_url: String,
  #[arg(long, env, hide_env_values = true)]
  pub meilisearch_master_key: String,
  #[arg(long, env = "API_SERVER_PORT")]
  pub bind: SocketAddr,
  #[arg(long, env = "API_SERVER_THREADS")]
  pub threads: Option<usize>
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let app_args = AppArg::parse();

  let mut builder = runtime::Builder::new_multi_thread();
  builder.enable_all();

  if let Some(j) = app_args.threads {
      builder.worker_threads(j);
  }

  let runtime = builder.build().map_err(|_| ApiServerError::TokioRuntime)?;

  // 指定したスレッド数でサーバーを実行する
  runtime
      .block_on(app::app(app_args.bind))
      .map_err(|_| ApiServerError::TokioRuntime)?;
  Ok(())
}
