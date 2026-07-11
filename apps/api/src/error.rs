//! The HTTP response envelope — both halves of one wire contract.
//! Success: `{ "success": true, "data": <payload> }` (payload always under
//! `data`, objects and arrays alike). Failure: `{ "success": false, "error" }`.
//! The Vue frontend reads `res.data` uniformly and keys off `res.success`.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use serde_json::json;

/// Success envelope. Named `Success` (not `Ok`) so it never shadows
/// `Result::Ok` in handlers; import it as `Success` everywhere (no aliases).
pub struct Success<T: Serialize>(pub T);

impl<T: Serialize> IntoResponse for Success<T> {
    fn into_response(self) -> Response {
        Json(json!({ "success": true, "data": self.0 })).into_response()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = Json(json!({ "success": false, "error": self.to_string() }));
        (self.status(), body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

// convenience for `?` on rusqlite / reqwest without manual mapping
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
