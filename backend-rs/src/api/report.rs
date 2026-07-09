use crate::state::AppState;
use axum::Router;

/// Endpoints consumed by the frontend (frontend/src/api/report.js + views):
///   POST /generate  /generate/status  /chat
///   GET  /{report_id}  /{report_id}/agent-log  /{report_id}/console-log
pub fn router() -> Router<AppState> {
    Router::new()
}
