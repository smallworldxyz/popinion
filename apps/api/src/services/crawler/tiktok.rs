//! TikTok crawler. TikTok server-injects a `__UNIVERSAL_DATA_FOR_REHYDRATION__`
//! JSON blob, but for plain HTTP clients it is usually empty or absent — the
//! real content is JS-rendered (see the ponytail note in fetch.rs). We parse
//! the blob when it carries video items; otherwise the crawl honestly reports
//! success=false until the fetch layer is swapped to the CDP browser.

use super::{extract_tagged, fetch::fetch_html};
use crate::models::{CrawlResult, ScrapedPost};
use chrono::{TimeZone, Utc};
use scraper::{Html, Selector};
use serde_json::Value;

const BASE_URL: &str = "https://www.tiktok.com";
const JS_WALL: &str =
    "tiktok.com returned no parseable videos (JS-rendered shell, empty rehydration blob); swap fetch.rs to the CDP browser at :9222/:9223 to enable this crawler";

/// Scrape a TikTok user's videos.
pub async fn crawl_user(username: &str, limit: usize) -> CrawlResult {
    let username = username.trim().trim_start_matches('@').to_string();
    let mut result = base_result(&format!("@{username}"));
    let mut url = reqwest::Url::parse(BASE_URL).expect("static base url");
    url.path_segments_mut().expect("base url has a path").push(&format!("@{username}"));
    match fetch_html(url.as_str()).await {
        Ok(html) => {
            result.posts = parse_videos(&html, limit);
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

/// Search TikTok for videos matching a query.
pub async fn crawl_query(query: &str, limit: usize) -> CrawlResult {
    let url = match reqwest::Url::parse_with_params(
        &format!("{BASE_URL}/search"),
        &[("q", query)],
    ) {
        Ok(u) => u,
        Err(e) => return failed(query, &format!("bad query: {e}")),
    };
    let mut result = base_result(query);
    match fetch_html(url.as_str()).await {
        Ok(html) => {
            result.posts = parse_videos(&html, limit);
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

fn base_result(query: &str) -> CrawlResult {
    CrawlResult {
        platform: "tiktok".into(),
        query: Some(query.to_string()),
        posts: vec![],
        users: vec![],
        crawled_at: Utc::now(),
        success: true,
        error: None,
    }
}

fn failed(query: &str, error: &str) -> CrawlResult {
    let mut r = base_result(query);
    r.success = false;
    r.error = Some(error.to_string());
    r
}

fn parse_videos(html: &str, limit: usize) -> Vec<ScrapedPost> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse(r#"script#__UNIVERSAL_DATA_FOR_REHYDRATION__"#).expect("static selector");
    let Some(blob) = doc.select(&sel).next().map(|e| e.text().collect::<String>()) else {
        return vec![];
    };
    let Ok(root) = serde_json::from_str::<Value>(&blob) else {
        return vec![];
    };
    let mut items = vec![];
    collect_items(&root, &mut items);
    items
        .into_iter()
        .take(limit)
        .map(video_to_post)
        .collect()
}

/// Recursively collect JSON objects that look like TikTok video items
/// (a caption `desc` plus a `stats` object).
fn collect_items<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
    match value {
        Value::Object(map) => {
            if map.get("desc").and_then(Value::as_str).is_some() && map.contains_key("stats") {
                out.push(value);
            }
            for v in map.values() {
                collect_items(v, out);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                collect_items(v, out);
            }
        }
        _ => {}
    }
}

fn video_to_post(item: &Value) -> ScrapedPost {
    let content = item.get("desc").and_then(Value::as_str).unwrap_or_default().to_string();
    let post_id = item.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
    let author = item.get("author");
    let author_id = author
        .and_then(|a| a.get("uniqueId"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let author_name = author
        .and_then(|a| a.get("nickname"))
        .and_then(Value::as_str)
        .unwrap_or(&author_id)
        .to_string();
    let stats = item.get("stats");
    let stat = |key: &str| {
        stats
            .and_then(|s| s.get(key))
            .and_then(Value::as_i64)
            .unwrap_or(0)
    };
    let timestamp = item
        .get("createTime")
        .and_then(Value::as_i64)
        .and_then(|t| Utc.timestamp_opt(t, 0).single())
        .unwrap_or_else(Utc::now);
    let url = (!author_id.is_empty() && !post_id.is_empty())
        .then(|| format!("{BASE_URL}/@{author_id}/video/{post_id}"));

    ScrapedPost {
        platform: "tiktok".into(),
        hashtags: extract_tagged(&content, '#'),
        mentions: extract_tagged(&content, '@'),
        post_id,
        content,
        author_id,
        author_name,
        timestamp,
        url,
        likes: stat("diggCount"),
        shares: stat("shareCount"),
        comments: stat("commentCount"),
        views: stat("playCount"),
        media_urls: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rehydration_blob() {
        let html = r#"<html><body>
        <script id="__UNIVERSAL_DATA_FOR_REHYDRATION__" type="application/json">
        {"__DEFAULT_SCOPE__":{"webapp.user-detail":{"itemList":[
          {"id":"7123","desc":"protest downtown #news @city","author":{"uniqueId":"reporter","nickname":"The Reporter"},
           "stats":{"diggCount":1200,"shareCount":45,"commentCount":88,"playCount":50000},"createTime":1714560600}
        ]}}}
        </script>
        </body></html>"#;
        let posts = parse_videos(html, 10);
        assert_eq!(posts.len(), 1);
        let p = &posts[0];
        assert_eq!(p.post_id, "7123");
        assert_eq!(p.author_id, "reporter");
        assert_eq!(p.author_name, "The Reporter");
        assert_eq!((p.likes, p.shares, p.comments, p.views), (1200, 45, 88, 50000));
        assert_eq!(p.hashtags, vec!["news"]);
        assert_eq!(p.mentions, vec!["city"]);
        assert_eq!(p.url.as_deref(), Some("https://www.tiktok.com/@reporter/video/7123"));
    }

    #[test]
    fn empty_blob_yields_no_posts() {
        let html = r#"<html><body>
        <script id="__UNIVERSAL_DATA_FOR_REHYDRATION__" type="application/json">{}</script>
        </body></html>"#;
        assert!(parse_videos(html, 10).is_empty());
    }
}
