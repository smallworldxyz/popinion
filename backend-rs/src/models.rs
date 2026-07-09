use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Success envelope matching the Vue frontend axios interceptor (`res.success`).
/// Named `Success` (not `Ok`) so it never shadows `Result::Ok` in handlers.
pub struct Success<T: Serialize>(pub T);

impl<T: Serialize> IntoResponse for Success<T> {
    fn into_response(self) -> Response {
        let mut v = serde_json::to_value(self.0).unwrap_or(Value::Null);
        // Merge `success: true` into object payloads; wrap non-objects under `data`.
        match &mut v {
            Value::Object(map) => {
                map.entry("success").or_insert(json!(true));
                Json(v).into_response()
            }
            _ => Json(json!({ "success": true, "data": v })).into_response(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Telegram,
    Twitter,
    Facebook,
    Youtube,
    Reddit,
    Web,
}

fn default_now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScrapedPost {
    pub platform: String,
    pub post_id: String,
    #[serde(default)]
    pub content: String,
    pub author_id: String,
    #[serde(default)]
    pub author_name: String,
    #[serde(default = "default_now")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub likes: i64,
    #[serde(default)]
    pub shares: i64,
    #[serde(default)]
    pub comments: i64,
    #[serde(default)]
    pub views: i64,
    #[serde(default)]
    pub media_urls: Vec<String>,
    #[serde(default)]
    pub hashtags: Vec<String>,
    #[serde(default)]
    pub mentions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScrapedUser {
    pub platform: String,
    pub user_id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub bio: String,
    #[serde(default)]
    pub profile_url: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub followers: i64,
    #[serde(default)]
    pub following: i64,
    #[serde(default)]
    pub post_count: i64,
    #[serde(default)]
    pub verified: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrawlResult {
    pub platform: String,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub posts: Vec<ScrapedPost>,
    #[serde(default)]
    pub users: Vec<ScrapedUser>,
    #[serde(default = "default_now")]
    pub crawled_at: chrono::DateTime<chrono::Utc>,
    #[serde(default = "crate::models::default_true")]
    pub success: bool,
    #[serde(default)]
    pub error: Option<String>,
}

pub fn default_true() -> bool {
    true
}

/// Back-compat re-export for subsystems written against the earlier `Ok` name.
/// A `use` alias (unlike a `type` alias) carries the tuple constructor, so
/// `models::Ok(x)` still builds. Use `Success` in new code; `Ok` shadows
/// `Result::Ok` if ever imported bare.
pub use self::Success as Ok;
