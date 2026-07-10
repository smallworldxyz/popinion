mod api;
mod config;
mod error;
mod llm;
mod models;
mod services;
mod settings;
mod sim;
mod state;

use axum::routing::get;
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use config::Config;
use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "popinion=debug,tower_http=warn,info".into()),
        )
        .init();

    let cfg = Config::from_env();
    let port = cfg.port;
    tracing::info!("Popinion Backend starting on :{port}");

    let state = AppState::new(cfg).await;

    // CORS mirrors Flask-CORS `origins: *` on /api/*.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .nest("/api/graph", api::graph::router())
        .nest("/api/simulation", api::simulation::router())
        .nest("/api/report", api::report::router())
        .nest("/api/crawl", api::crawl::router())
        .nest("/api/settings", api::settings::router())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("bind port");
    tracing::info!("Popinion Backend listening on http://0.0.0.0:{port}");
    axum::serve(listener, app).await.expect("server");
}

async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "status": "ok", "service": "Popinion Backend" }))
}
