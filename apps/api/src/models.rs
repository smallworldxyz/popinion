//! Crawler-domain structs: raw crawl records (reality-seed *inputs*, never
//! Personas). The HTTP response envelope lives in `crate::error`.

use serde::{Deserialize, Serialize};

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
