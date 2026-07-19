//! Threads (threads.net) crawler. Threads server-embeds post data in
//! `<script type="application/json">` blobs, but for plain HTTP clients the
//! content is JS-rendered (see the ponytail note in fetch.rs). We parse the
//! embedded JSON when it carries posts; otherwise the crawl honestly reports
//! success=false until the fetch layer is swapped to the CDP browser.

use super::{extract_tagged, fetch::fetch_html};
use crate::models::{CrawlResult, ScrapedPost};
use chrono::{TimeZone, Utc};
use scraper::{Html, Selector};
use serde_json::Value;

const BASE_URL: &str = "https://www.threads.net";
const JS_WALL: &str =
    "threads.net returned no parseable posts (JS-rendered shell, empty embedded JSON); swap fetch.rs to the CDP browser at :9222/:9223 to enable this crawler";

/// Scrape a Threads user's posts.
pub async fn crawl_user(username: &str, limit: usize) -> CrawlResult {
    let username = username.trim().trim_start_matches('@').to_string();
    let mut result = CrawlResult {
        platform: "threads".into(),
        query: Some(format!("@{username}")),
        posts: vec![],
        users: vec![],
        crawled_at: Utc::now(),
        success: true,
        error: None,
    };
    match fetch_html(&format!("{BASE_URL}/@{username}")).await {
        Ok(html) => {
            result.posts = parse_posts(&html, &username, limit);
            if result.posts.is_empty() {
                result.success = false;
                result.error = Some(JS_WALL.into());
            }
        }
        Err(e) => {
            result.success = false;
            result.error = Some(format!("fetch failed: {e}"));
        }
    }
    result
}

fn parse_posts(html: &str, username: &str, limit: usize) -> Vec<ScrapedPost> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse(r#"script[type="application/json"]"#).expect("static selector");
    let mut items: Vec<Value> = vec![];
    for script in doc.select(&sel) {
        let blob = script.text().collect::<String>();
        if let Ok(root) = serde_json::from_str::<Value>(&blob) {
            collect_posts(&root, &mut items);
        }
    }
    items
        .iter()
        .take(limit)
        .map(|item| post_to_scraped(item, username))
        .collect()
}

/// Recursively collect JSON objects that look like Threads posts (a `caption`
/// object carrying a `text` string, alongside a `code` permalink token).
fn collect_posts(value: &Value, out: &mut Vec<Value>) {
    match value {
        Value::Object(map) => {
            let has_caption = map
                .get("caption")
                .and_then(|c| c.get("text"))
                .and_then(Value::as_str)
                .is_some();
            if has_caption && map.contains_key("code") {
                out.push(value.clone());
            }
            for v in map.values() {
                collect_posts(v, out);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_posts(v, out);
            }
        }
        _ => {}
    }
}

fn post_to_scraped(item: &Value, default_user: &str) -> ScrapedPost {
    let content = item
        .get("caption")
        .and_then(|c| c.get("text"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let code = item.get("code").and_then(Value::as_str).unwrap_or_default().to_string();
    let post_id = item
        .get("pk")
        .and_then(|p| p.as_str().map(str::to_string).or_else(|| p.as_i64().map(|n| n.to_string())))
        .unwrap_or_else(|| code.clone());
    let user = item.get("user");
    let author_id = user
        .and_then(|u| u.get("username"))
        .and_then(Value::as_str)
        .unwrap_or(default_user)
        .to_string();
    let author_name = user
        .and_then(|u| u.get("full_name"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(&author_id)
        .to_string();
    let timestamp = item
        .get("taken_at")
        .and_then(Value::as_i64)
        .and_then(|t| Utc.timestamp_opt(t, 0).single())
        .unwrap_or_else(Utc::now);
    let likes = item.get("like_count").and_then(Value::as_i64).unwrap_or(0);

    ScrapedPost {
        platform: "threads".into(),
        hashtags: extract_tagged(&content, '#'),
        mentions: extract_tagged(&content, '@'),
        post_id,
        content,
        author_id,
        author_name,
        timestamp,
        url: (!code.is_empty()).then(|| format!("{BASE_URL}/post/{code}")),
        likes,
        shares: 0,
        comments: 0,
        views: 0,
        media_urls: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_embedded_post_json() {
        let html = r#"<html><body>
        <script type="application/json">
        {"data":{"thread_items":[
          {"post":{"pk":"3141592","code":"CxYz","caption":{"text":"markets rally today #econ @gov"},
            "user":{"username":"analyst","full_name":"The Analyst"},"like_count":230,"taken_at":1714560600}}
        ]}}
        </script>
        </body></html>"#;
        let posts = parse_posts(html, "analyst", 10);
        assert_eq!(posts.len(), 1);
        let p = &posts[0];
        assert_eq!(p.post_id, "3141592");
        assert_eq!(p.author_id, "analyst");
        assert_eq!(p.author_name, "The Analyst");
        assert_eq!(p.content, "markets rally today #econ @gov");
        assert_eq!(p.likes, 230);
        assert_eq!(p.hashtags, vec!["econ"]);
        assert_eq!(p.mentions, vec!["gov"]);
        assert_eq!(p.url.as_deref(), Some("https://www.threads.net/post/CxYz"));
    }

    #[test]
    fn no_json_yields_no_posts() {
        let html = r#"<html><body><div>JS shell only</div></body></html>"#;
        assert!(parse_posts(html, "analyst", 10).is_empty());
    }
}
