//! Google Trends crawler. The daily-trends RSS feed is server-rendered XML
//! (no JS wall), so these crawls return real data over plain HTTP. The `ht:`
//! namespaced tags parse unreliably through `scraper`, so we extract fields
//! with lightweight manual slicing between known tag boundaries.

use super::fetch::fetch_html;
use crate::models::{CrawlResult, ScrapedPost};
use chrono::Utc;

const BASE_URL: &str = "https://trends.google.com/trending/rss";
const EMPTY: &str = "google trends RSS returned no trending items (empty feed or blocked); retry later or check the geo code";

/// Crawl the current daily trending terms for a geo (e.g. "US").
pub async fn crawl_trends(geo: &str, limit: usize) -> CrawlResult {
    let geo = {
        let g = geo.trim();
        if g.is_empty() { "US".to_string() } else { g.to_uppercase() }
    };
    let url = reqwest::Url::parse_with_params(BASE_URL, &[("geo", geo.as_str())])
        .expect("static base url");
    let mut result = CrawlResult {
        platform: "google_trends".into(),
        query: Some(geo.clone()),
        posts: vec![],
        users: vec![],
        crawled_at: Utc::now(),
        success: true,
        error: None,
    };
    match fetch_html(url.as_str()).await {
        Ok(xml) => {
            result.posts = parse_trends(&xml, &geo, limit);
            if result.posts.is_empty() {
                result.success = false;
                result.error = Some(EMPTY.into());
            }
        }
        Err(e) => {
            result.success = false;
            result.error = Some(format!("fetch failed: {e}"));
        }
    }
    result
}

/// Slice the text between the first `<tag>` and its matching `</tag>` inside
/// `item`, stripping an optional CDATA wrapper.
fn tag_value<'a>(item: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = item.find(&open)? + open.len();
    let end = item[start..].find(&close)? + start;
    let mut inner = item[start..end].trim();
    inner = inner
        .strip_prefix("<![CDATA[")
        .and_then(|s| s.strip_suffix("]]>"))
        .map(str::trim)
        .unwrap_or(inner);
    Some(inner)
}

fn parse_trends(xml: &str, geo: &str, limit: usize) -> Vec<ScrapedPost> {
    xml.split("<item>")
        .skip(1)
        .filter_map(|chunk| chunk.split("</item>").next())
        .take(limit)
        .enumerate()
        .filter_map(|(i, item)| {
            let term = tag_value(item, "title")?.trim();
            if term.is_empty() {
                return None;
            }
            let traffic = tag_value(item, "ht:approx_traffic").unwrap_or("").trim();
            let snippet = tag_value(item, "ht:news_item_snippet").unwrap_or("").trim();
            let news_url = tag_value(item, "ht:news_item_url").map(str::to_string);

            let mut content = term.to_string();
            if !traffic.is_empty() {
                content.push_str(&format!(" ({traffic} searches)"));
            }
            if !snippet.is_empty() {
                content.push_str(&format!(" — {snippet}"));
            }

            Some(ScrapedPost {
                platform: "google_trends".into(),
                post_id: format!("{geo}_{i}"),
                content,
                author_id: "google_trends".into(),
                author_name: "google_trends".into(),
                timestamp: Utc::now(),
                url: news_url,
                likes: 0,
                shares: 0,
                comments: 0,
                views: super::parse_count(traffic),
                media_urls: vec![],
                hashtags: vec![],
                mentions: vec![],
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_trends_rss() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <rss version="2.0">
        <channel>
          <title>Daily Search Trends</title>
          <item>
            <title>Election results</title>
            <ht:approx_traffic>200K+</ht:approx_traffic>
            <ht:news_item>
              <ht:news_item_title>Live: national tally</ht:news_item_title>
              <ht:news_item_url>https://news.example.com/tally</ht:news_item_url>
              <ht:news_item_snippet>Counting continues into the night.</ht:news_item_snippet>
            </ht:news_item>
          </item>
          <item>
            <title>Local derby</title>
            <ht:approx_traffic>50K+</ht:approx_traffic>
          </item>
        </channel>
        </rss>"#;
        let posts = parse_trends(xml, "US", 10);
        assert_eq!(posts.len(), 2);
        let p = &posts[0];
        assert_eq!(p.post_id, "US_0");
        assert_eq!(p.author_name, "google_trends");
        assert!(p.content.starts_with("Election results"));
        assert!(p.content.contains("200K+ searches"));
        assert!(p.content.contains("Counting continues into the night."));
        assert_eq!(p.url.as_deref(), Some("https://news.example.com/tally"));
        assert_eq!(p.views, 200_000);
        // Second item has no news snippet, just the term + traffic.
        assert_eq!(posts[1].content, "Local derby (50K+ searches)");
    }
}
