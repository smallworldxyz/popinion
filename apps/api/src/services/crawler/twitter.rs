//! Twitter/X crawler. Full parse pipeline for X's server markup
//! (article[data-testid="tweet"]), but X serves a JS shell to plain HTTP
//! clients — see the ponytail note in fetch.rs for the swap point that
//! lifts this. Until then these crawls honestly report success=false.

use super::{extract_tagged, fetch::fetch_html, parse_count};
use crate::models::{CrawlResult, ScrapedPost, ScrapedUser};
use chrono::{DateTime, Utc};
use scraper::{Html, Selector};

const BASE_URL: &str = "https://x.com";
const JS_WALL: &str =
    "x.com returned no parseable tweets (JS-rendered shell); swap fetch.rs to the CDP browser at :9222/:9223 to enable this crawler";

fn sel(css: &str) -> Selector {
    Selector::parse(css).expect("static selector")
}

/// Search X for tweets matching a query.
pub async fn crawl_query(query: &str, limit: usize) -> CrawlResult {
    let url = match reqwest::Url::parse_with_params(
        &format!("{BASE_URL}/search"),
        &[("q", query), ("src", "typed_query"), ("f", "live")],
    ) {
        Ok(u) => u,
        Err(e) => return failed(query, &format!("bad query: {e}")),
    };
    let mut result = base_result(query);
    match fetch_html(url.as_str()).await {
        Ok(html) => {
            result.posts = parse_tweets(&html, None, limit);
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

/// Scrape a user's profile page: their tweets plus a profile card.
pub async fn crawl_user(username: &str, limit: usize) -> CrawlResult {
    let username = username.trim().trim_start_matches('@').to_string();
    let mut result = base_result(&format!("@{username}"));
    match fetch_html(&format!("{BASE_URL}/{username}")).await {
        Ok(html) => {
            let (posts, user) = parse_profile(&html, &username, limit);
            result.posts = posts;
            result.users.extend(user);
            if result.posts.is_empty() && result.users.is_empty() {
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
        platform: "twitter".into(),
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

fn parse_profile(html: &str, username: &str, limit: usize) -> (Vec<ScrapedPost>, Option<ScrapedUser>) {
    let posts = parse_tweets(html, Some(username), limit);
    let doc = Html::parse_document(html);

    let name = doc
        .select(&sel(r#"[data-testid="UserName"]"#))
        .next()
        .map(|e| e.text().collect::<String>());
    let user = name.map(|name_text| {
        let display_name = name_text.lines().next().unwrap_or(username).trim().to_string();
        let bio = doc
            .select(&sel(r#"[data-testid="UserDescription"]"#))
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let (mut followers, mut following) = (0, 0);
        for a in doc.select(&sel(r#"a[href*="followers"], a[href*="following"]"#)) {
            let href = a.value().attr("href").unwrap_or_default().to_lowercase();
            let count = parse_count(&a.text().collect::<String>());
            if href.contains("followers") {
                followers = count;
            } else if href.contains("following") {
                following = count;
            }
        }

        ScrapedUser {
            platform: "twitter".into(),
            user_id: username.to_string(),
            username: username.to_string(),
            display_name,
            bio,
            profile_url: Some(format!("{BASE_URL}/{username}")),
            avatar_url: None,
            followers,
            following,
            post_count: 0,
            verified: false,
        }
    });

    (posts, user)
}

fn parse_tweets(html: &str, default_author: Option<&str>, limit: usize) -> Vec<ScrapedPost> {
    let doc = Html::parse_document(html);
    let tweet_sel = sel(r#"article[data-testid="tweet"]"#);
    let text_sel = sel(r#"[data-testid="tweetText"]"#);
    let user_sel = sel(r#"[data-testid="User-Name"]"#);
    let time_sel = sel("time");
    let stats_sel = sel(r#"[data-testid="app-text-transition-container"]"#);
    let img_sel = sel(r#"img[src*="media"]"#);

    let mut posts = vec![];
    for el in doc.select(&tweet_sel).take(limit) {
        let content = el
            .select(&text_sel)
            .next()
            .map(|t| t.text().collect::<String>())
            .unwrap_or_default()
            .trim()
            .to_string();

        // "Display Name\n@username" block.
        let (mut author_name, mut author_id) = (
            default_author.unwrap_or("Unknown").to_string(),
            default_author.unwrap_or("unknown").to_string(),
        );
        if let Some(u) = el.select(&user_sel).next() {
            let text = u.text().collect::<String>();
            let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
            if let Some(first) = lines.next() {
                author_name = first.to_string();
            }
            if let Some(handle) = lines.find(|l| l.starts_with('@')) {
                author_id = handle.trim_start_matches('@').to_string();
            }
        }

        let mut timestamp = Utc::now();
        let mut url = None;
        if let Some(t) = el.select(&time_sel).next() {
            if let Some(d) = t.value().attr("datetime") {
                if let Ok(parsed) = DateTime::parse_from_rfc3339(d) {
                    timestamp = parsed.with_timezone(&Utc);
                }
            }
            // <time> sits inside the permalink anchor.
            url = t
                .parent()
                .and_then(scraper::ElementRef::wrap)
                .and_then(|a| a.value().attr("href"))
                .map(|href| format!("{BASE_URL}{href}"));
        }

        let post_id = url
            .as_deref()
            .and_then(|u| u.rsplit('/').next())
            .unwrap_or_default()
            .to_string();

        // Stat containers are ordered replies, retweets, likes.
        let stats: Vec<i64> = el
            .select(&stats_sel)
            .map(|s| parse_count(&s.text().collect::<String>()))
            .collect();
        let comments = stats.first().copied().unwrap_or(0);
        let shares = stats.get(1).copied().unwrap_or(0);
        let likes = stats.get(2).copied().unwrap_or(0);

        let media_urls: Vec<String> = el
            .select(&img_sel)
            .filter_map(|img| img.value().attr("src"))
            .map(str::to_string)
            .collect();

        if content.is_empty() {
            continue;
        }
        posts.push(ScrapedPost {
            platform: "twitter".into(),
            hashtags: extract_tagged(&content, '#'),
            mentions: extract_tagged(&content, '@'),
            post_id,
            content,
            author_id,
            author_name,
            timestamp,
            url,
            likes,
            shares,
            comments,
            views: 0,
            media_urls,
        });
    }
    posts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tweet_markup() {
        let html = r#"
        <article data-testid="tweet">
          <div data-testid="User-Name">Jane Doe
@janedoe</div>
          <a href="/janedoe/status/9876543210"><time datetime="2024-06-01T12:00:00.000Z"></time></a>
          <div data-testid="tweetText">Rust is great #rustlang cc @ferris</div>
          <span data-testid="app-text-transition-container">12</span>
          <span data-testid="app-text-transition-container">34</span>
          <span data-testid="app-text-transition-container">1.2K</span>
        </article>"#;
        let posts = parse_tweets(html, None, 10);
        assert_eq!(posts.len(), 1);
        let p = &posts[0];
        assert_eq!(p.author_name, "Jane Doe");
        assert_eq!(p.author_id, "janedoe");
        assert_eq!(p.post_id, "9876543210");
        assert_eq!((p.comments, p.shares, p.likes), (12, 34, 1200));
        assert_eq!(p.hashtags, vec!["rustlang"]);
        assert_eq!(p.mentions, vec!["ferris"]);
    }
}
