use crate::state::AppState;
use axum::Router;

/// Endpoints consumed by the frontend (frontend/src/api/simulation.js + views):
///   POST /create  /prepare  /prepare/preview  /prepare/status
///   POST /start  /stop  /close-env  /env-status
///   GET  /list  /{id}  /{id}/config  /{id}/config/realtime
///   GET  /{id}/profiles  /{id}/profiles/realtime
///   GET  /{id}/run-status  /{id}/run-status/detail
///   GET  /{id}/posts  /{id}/timeline  /{id}/actions  /{id}/agent-stats
///   POST /interview/batch  /panel-chat
///   POST /survey/create  /survey/deploy   GET /survey/list  /survey/{id}
pub fn router() -> Router<AppState> {
    Router::new()
}
