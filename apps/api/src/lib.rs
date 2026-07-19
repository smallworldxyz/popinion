//! Popinion backend as a library, so it can run standalone (the `popinion` bin)
//! or in-process inside the Tauri desktop shell. `build_app` returns the full
//! router (API + optionally the built frontend on the same origin).

pub mod api;
pub mod chatgpt_auth;
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
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
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
    restore_reports(&state);

    // Scope CORS to the local dev frontend origin only. The desktop app is
    // same-origin (served from static_dir) and Vite proxies /api server-side,
    // so no wildcard is needed — and a wildcard on this unauthenticated API
    // would let any website the user visits drive it cross-origin.
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list([
            "http://localhost:3000".parse().unwrap(),
            "http://127.0.0.1:3000".parse().unwrap(),
        ]))
        .allow_methods(Any)
        .allow_headers(Any);

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

/// Reload previously generated reports into the in-process registry, so a
/// finished run still answers /api/report/check after a restart instead of
/// silently offering to regenerate.
fn restore_reports(state: &AppState) {
    let mgr = state.sim_manager();
    let mut n = 0;
    for meta in mgr.list() {
        let path = mgr.report_path(&meta.simulation_id);
        if path.exists()
            && services::report::registry::restore(
                &path,
                mgr.db_path(&meta.simulation_id).to_string_lossy().into_owned(),
            )
            .is_some()
        {
            n += 1;
        }
    }
    if n > 0 {
        tracing::info!("restored {n} report(s) from disk");
    }
}

/// Serve the app on an already-bound listener until shutdown.
pub async fn serve(listener: tokio::net::TcpListener, cfg: Config) {
    let app = build_app(cfg).await;
    axum::serve(listener, app).await.expect("server");
}

pub async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "status": "ok", "service": "Popinion Backend" }))
}
