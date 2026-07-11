//! Facebook public-page crawler (m.facebook.com). Facebook gates almost all
//! content behind login/JS for plain HTTP clients — see the ponytail note in
//! fetch.rs. Crawls report success=false until the fetch layer is swapped
//! to the CDP browser.

use super::fetch::fetch_html;
use crate::models::{CrawlResult, ScrapedPost};
use chrono::Utc;
use scraper::{Html, Selector};

const BASE_URL: &str = "https://m.facebook.com";
const JS_WALL: &str =
    "m.facebook.com returned no parseable posts (login/JS wall); swap fetch.rs to the CDP browser at :9222/:9223 to enable this crawler";

/// Crawl a public Facebook page's posts.
pub async fn crawl_page(page: &str, limit: usize) -> CrawlResult {
    let page_id = page.trim().trim_start_matches('/').to_string();
    let mut result = CrawlResult {
        platform: "facebook".into(),
        query: Some(page_id.clone()),
        posts: vec![],
        users: vec![],
        crawled_at: Utc::now(),
        success: true,
        error: None,
    };
    match fetch_html(&format!("{BASE_URL}/{page_id}")).await {
        Ok(html) => {
            result.posts = parse_posts(&html, &page_id, limit);
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

fn parse_posts(html: &str, page_id: &str, limit: usize) -> Vec<ScrapedPost> {
    let doc = Html::parse_document(html);
    let post_sel = Selector::parse(r#"article, [role="article"], [data-ft]"#).expect("static selector");

    let mut posts = vec![];
    for (i, el) in doc.select(&post_sel).take(limit).enumerate() {
        let content = el
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if content.is_empty() {
            continue;
        }
        posts.push(ScrapedPost {
            platform: "facebook".into(),
            post_id: format!("{page_id}_{i}"),
            content,
            author_id: page_id.to_string(),
            author_name: page_id.to_string(),
            timestamp: Utc::now(),
            url: Some(format!("{BASE_URL}/{page_id}")),
            likes: 0,
            shares: 0,
            comments: 0,
            views: 0,
            media_urls: vec![],
            hashtags: vec![],
            mentions: vec![],
        });
    }
    posts
}
