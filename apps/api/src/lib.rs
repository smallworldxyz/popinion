//! Popinion backend as a library, so it can run standalone (the `popinion` bin)
//! or in-process inside the Tauri desktop shell. `build_app` returns the full
//! router (API + optionally the built frontend on the same origin).

pub mod api;
pub mod config;
pub mod error;
pub mod llm;
pub mod models;
pub mod services;
pub mod settings;
pub mod sim;
pub mod state;

use axum::routing::get;
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

pub use config::Config;
pub use state::AppState;

/// Build the application router: the JSON API and, when `static_dir` is set, the
/// built frontend served on the SAME origin (SPA fallback to index.html). One
/// origin means the desktop app needs no separate dev server and no CORS.
pub async fn build_app(cfg: Config) -> Router {
    let static_dir = cfg.static_dir.clone();
    let state = AppState::new(cfg).await;

    // CORS stays permissive for the browser-dev deployment (Vite on another port).
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    let mut app = Router::new()
        .route("/health", get(health))
        .nest("/api/graph", api::graph::router())
        .nest("/api/simulation", api::simulation::router())
        .nest("/api/report", api::report::router())
        .nest("/api/crawl", api::crawl::router())
        .nest("/api/settings", api::settings::router());

    if let Some(dir) = static_dir {
        let index = std::path::Path::new(&dir).join("index.html");
        app = app.fallback_service(ServeDir::new(&dir).fallback(ServeFile::new(index)));
    }

    app.layer(cors).layer(TraceLayer::new_for_http()).with_state(state)
}

/// Bind an ephemeral loopback port for the in-process desktop server. Returns
/// the listener and the port so the shell can point its window at it.
pub async fn bind_loopback() -> std::io::Result<(tokio::net::TcpListener, u16)> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();
    Ok((listener, port))
}

/// Serve the app on an already-bound listener until shutdown.
pub async fn serve(listener: tokio::net::TcpListener, cfg: Config) {
    let app = build_app(cfg).await;
    axum::serve(listener, app).await.expect("server");
}

pub async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "status": "ok", "service": "Popinion Backend" }))
}
