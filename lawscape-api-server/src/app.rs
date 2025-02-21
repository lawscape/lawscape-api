use crate::lawscape_api_server_error::ApiServerError;
use axum::{Router, extract::Query, http::Method, response::Json, routing::get};
use lawscape_core::{LegalDocumentDependencies, LegalDocumentsRegistory};
use reqwest::header::CONTENT_TYPE;
use std::collections::HashMap;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

/// ログを出力するための設定など
async fn init_logger() -> Result<(), ApiServerError> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|_| ApiServerError::LoggerConfig)?;
    Ok(())
}

pub async fn app(
    bind: SocketAddr,
    meilisearch_url: String,
    meilisearch_master_key: String,
) -> Result<(), ApiServerError> {
    init_logger().await?;

    let app = Router::new()
        .route(
            "/v1/ping",
            get(|| async {
                info!("GET /ping");
                "pong"
            }),
        )
        .route(
            "/v1/search",
            get(move |query: Query<HashMap<String, String>>| {
                let search_word = query.0.get("word").cloned().unwrap_or_default();
                info!("GET /v1/search: {search_word}");
                v1_get_search(search_word, meilisearch_url, meilisearch_master_key)
            }),
        )
        .layer(
            CorsLayer::new()
                .allow_methods([Method::GET])
                .allow_headers([CONTENT_TYPE])
                .allow_origin(Any),
        );
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|_| ApiServerError::AxumError)?;

    info!("server start");

    axum::serve(listener, app)
        .await
        .map_err(|_| ApiServerError::AxumError)?;

    info!("server down");
    Ok(())
}

async fn v1_get_search(
    word: String,
    meilisearch_url: String,
    meilisearch_master_key: String,
) -> Result<Json<Vec<LegalDocumentDependencies>>, ApiServerError> {
    let search_registry = LegalDocumentsRegistory::new(&meilisearch_url, &meilisearch_master_key)
        .map_err(|_| ApiServerError::MeilisearchError)?;
    if word.is_empty() {
        Err(ApiServerError::SearchError)
    } else {
        let search_result = search_registry
            .search(&word)
            .await
            .map_err(|_| ApiServerError::SearchError)?;
        let dependencies_result = lawscape_core::analyze_search_result_dependencies(&search_result);
        let result = dependencies_result
            .values()
            .cloned()
            .collect::<Vec<LegalDocumentDependencies>>();
        Ok(Json(result))
    }
}
