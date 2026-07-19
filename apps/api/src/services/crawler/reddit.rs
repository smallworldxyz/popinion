//! Reddit crawler. Reddit exposes public JSON endpoints that are server-served
//! (no JS wall), so these crawls return real data over plain HTTP. When Reddit
//! blocks a request it returns an HTML error/rate-limit page instead of JSON;
//! that fails the JSON parse and the crawl honestly reports success=false.

use super::{extract_tagged, fetch::fetch_html};
use crate::models::{CrawlResult, ScrapedPost};
use chrono::{TimeZone, Utc};
use serde::Deserialize;

const BASE_URL: &str = "https://www.reddit.com";
const BLOCKED: &str =
    "reddit returned no parseable JSON (rate-limited or blocked HTML page); retry later or route through the CDP browser at :9222/:9223";

#[derive(Deserialize)]
struct Listing {
    data: ListingData,
}

#[derive(Deserialize)]
struct ListingData {
    #[serde(default)]
    children: Vec<Child>,
}

#[derive(Deserialize)]
struct Child {
    data: PostData,
}

#[derive(Deserialize)]
struct PostData {
    #[serde(default)]
    title: String,
    #[serde(default)]
    selftext: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    ups: i64,
    #[serde(default)]
    num_comments: i64,
    #[serde(default)]
    permalink: String,
    #[serde(default)]
    created_utc: f64,
    #[serde(default)]
    id: String,
}

/// Crawl the top posts of a subreddit over the past week.
pub async fn crawl_subreddit(subreddit: &str, limit: usize) -> CrawlResult {
    let sub = subreddit.trim().trim_start_matches("r/").trim_start_matches('/').to_string();
    let url = format!("{BASE_URL}/r/{sub}/top.json?t=week&limit={limit}");
    crawl(&url, &sub).await
}

/// Search Reddit for the top posts matching a query.
pub async fn crawl_query(query: &str, limit: usize) -> CrawlResult {
    let url = match reqwest::Url::parse_with_params(
        &format!("{BASE_URL}/search.json"),
        &[
            ("q", query),
            ("sort", "top"),
            ("limit", &limit.to_string()),
        ],
    ) {
        Ok(u) => u,
        Err(e) => return failed(query, &format!("bad query: {e}")),
    };
    crawl(url.as_str(), query).await
}

async fn crawl(url: &str, hint: &str) -> CrawlResult {
    let mut result = base_result(hint);
    match fetch_html(url).await {
        Ok(body) => match parse_listing(&body, usize::MAX) {
            Ok(posts) if !posts.is_empty() => result.posts = posts,
            _ => {
                result.success = false;
                result.error = Some(BLOCKED.into());
            }
        },
        Err(e) => {
            result.success = false;
            result.error = Some(format!("fetch failed: {e}"));
        }
    }
    result
}

fn base_result(query: &str) -> CrawlResult {
    CrawlResult {
        platform: "reddit".into(),
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

fn parse_listing(body: &str, limit: usize) -> anyhow::Result<Vec<ScrapedPost>> {
    let listing: Listing = serde_json::from_str(body)?;
    let posts = listing
        .data
        .children
        .into_iter()
        .take(limit)
        .filter_map(|c| {
            let d = c.data;
            let content = match (d.title.trim(), d.selftext.trim()) {
                ("", "") => return None,
                (t, "") => t.to_string(),
                ("", s) => s.to_string(),
                (t, s) => format!("{t}\n\n{s}"),
            };
            let timestamp = Utc
                .timestamp_opt(d.created_utc as i64, 0)
                .single()
                .unwrap_or_else(Utc::now);
            Some(ScrapedPost {
                platform: "reddit".into(),
                hashtags: extract_tagged(&content, '#'),
                mentions: extract_tagged(&content, '@'),
                post_id: d.id,
                content,
                author_id: d.author.clone(),
                author_name: d.author,
                timestamp,
                url: (!d.permalink.is_empty()).then(|| format!("{BASE_URL}{}", d.permalink)),
                likes: d.ups,
                shares: 0,
                comments: d.num_comments,
                views: 0,
                media_urls: vec![],
            })
        })
        .collect();
    Ok(posts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_listing_json() {
        let body = r#"{
          "data": {
            "children": [
              {"data": {
                "id": "abc123",
                "title": "Fuel subsidy ends next year",
                "selftext": "Details in the comments #policy",
                "author": "citizen_x",
                "ups": 1520,
                "num_comments": 340,
                "permalink": "/r/news/comments/abc123/fuel_subsidy/",
                "created_utc": 1714560600.0,
                "subreddit": "news"
              }},
              {"data": {
                "id": "empty1",
                "title": "",
                "selftext": "",
                "author": "ghost"
              }}
            ]
          }
        }"#;
        let posts = parse_listing(body, 10).unwrap();
        assert_eq!(posts.len(), 1);
        let p = &posts[0];
        assert_eq!(p.post_id, "abc123");
        assert_eq!(p.author_name, "citizen_x");
        assert!(p.content.starts_with("Fuel subsidy ends next year"));
        assert!(p.content.contains("Details in the comments"));
        assert_eq!(p.likes, 1520);
        assert_eq!(p.comments, 340);
        assert_eq!(p.hashtags, vec!["policy"]);
        assert_eq!(p.url.as_deref(), Some("https://www.reddit.com/r/news/comments/abc123/fuel_subsidy/"));
    }
}
