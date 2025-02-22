use crate::lawscape_api_server_error::ApiServerError;
use axum::{Router, extract::Query, http::Method, response::Json, routing::get};
use lawscape_core::{LegalDocumentDependencies, LegalDocumentsRegistory};
use reqwest::header::CONTENT_TYPE;
use std::collections::HashMap;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};

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
    default_limit: usize,
    default_search_cancell_score: f64,
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
                let limit = query
                    .0
                    .get("limit")
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(default_limit);
                let search_cancell_score = query
                    .0
                    .get("cancell_score")
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(default_search_cancell_score);
                info!("GET /v1/search: {search_word}");
                v1_get_search(
                    search_word,
                    meilisearch_url,
                    meilisearch_master_key,
                    limit,
                    search_cancell_score,
                )
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
    limit: usize,
    search_cancell_score: f64,
) -> Result<Json<Vec<LegalDocumentDependencies>>, ApiServerError> {
    let search_registry = LegalDocumentsRegistory::new(&meilisearch_url, &meilisearch_master_key)
        .map_err(|e| {
        error!("failed at LegalDocumentsRegistory::new; {e}");
        ApiServerError::MeilisearchError
    })?;
    if word.is_empty() {
        error!("search word is empty");
        Err(ApiServerError::SearchError)
    } else {
        let search_result = search_registry
            .search(&word, limit, search_cancell_score)
            .await
            .map_err(|e| {
                error!("failed at search; {e}");
                ApiServerError::SearchError
            })?;
        let dependencies_result = lawscape_core::analyze_search_result_dependencies(&search_result);
        let result = dependencies_result
            .values()
            .cloned()
            .collect::<Vec<LegalDocumentDependencies>>();
        Ok(Json(result))
    }
}
