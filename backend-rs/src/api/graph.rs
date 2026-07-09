use crate::state::AppState;
use axum::Router;

/// Endpoints consumed by the frontend (frontend/src/api/graph.js + views):
///   POST   /ontology/generate
///   POST   /build
///   GET    /task/{task_id}
///   GET    /data/{graph_id}
///   DELETE /delete/{graph_id}
///   GET    /projects
///   GET    /project/{project_id}
///   DELETE /project/{project_id}
pub fn router() -> Router<AppState> {
    Router::new()
}
