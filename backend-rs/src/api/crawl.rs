use crate::state::AppState;
use axum::Router;

/// Endpoints (frontend/src/api + crawl UI):
///   POST /telegram  /twitter  /facebook  /bridge
///   GET  /results  /results/{filename}
pub fn router() -> Router<AppState> {
    Router::new()
}
