use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("not implemented: {0}")]
    NotImplemented(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            AppError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// The Vue frontend's axios interceptor keys off `success` + `error`, so every
// error response carries that envelope.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = Json(json!({ "success": false, "error": self.to_string() }));
        (self.status(), body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

// convenience for `?` on rusqlite / neo4rs / reqwest without manual mapping
impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Other(anyhow::Error::new(e))
    }
}
impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Other(anyhow::Error::new(e))
    }
}
impl From<neo4rs::Error> for AppError {
    fn from(e: neo4rs::Error) -> Self {
        AppError::Other(anyhow::Error::new(e))
    }
}
