//! Telegram public-channel crawler (t.me/s/<channel> preview pages).
//! Fully functional over plain HTTP: the preview pages are server-rendered.

use super::{fetch::fetch_html, parse_count};
use crate::models::{CrawlResult, ScrapedPost, ScrapedUser};
use chrono::{DateTime, Utc};
use scraper::{Html, Selector};

const BASE_URL: &str = "https://t.me/s";

fn sel(css: &str) -> Selector {
    Selector::parse(css).expect("static selector")
}

/// Crawl a public Telegram channel. Errors are folded into the result
/// (`success=false` + `error`), mirroring the Python BaseCrawler.crawl().
pub async fn crawl_channel(channel: &str, limit: usize) -> CrawlResult {
    let channel = channel.trim().trim_start_matches('@').to_string();
    let mut result = CrawlResult {
        platform: "telegram".into(),
        query: Some(channel.clone()),
        posts: vec![],
        users: vec![],
        crawled_at: Utc::now(),
        success: true,
        error: None,
    };

    let mut before: Option<String> = None;
    // t.me/s serves ~20 posts per page; paginate with ?before=<oldest id>.
    for _ in 0..10 {
        let url = match &before {
            Some(id) => format!("{BASE_URL}/{channel}?before={id}"),
            None => format!("{BASE_URL}/{channel}"),
        };
        let html = match fetch_html(&url).await {
            Ok(h) => h,
            Err(e) => {
                if result.posts.is_empty() {
                    result.success = false;
                    result.error = Some(format!("fetch failed: {e}"));
                }
                break;
            }
        };
        let page = parse_page(&html, &channel);
        if result.users.is_empty() {
            result.users.extend(page.user);
        }
        if page.posts.is_empty() {
            break;
        }
        // Pages are newest-first; oldest id on the page drives pagination.
        let oldest = page
            .posts
            .iter()
            .filter_map(|p| p.post_id.parse::<i64>().ok())
            .min();
        result.posts.extend(page.posts);
        if result.posts.len() >= limit {
            break;
        }
        match oldest {
            Some(id) if id > 1 => before = Some(id.to_string()),
            _ => break,
        }
    }

    result.posts.truncate(limit);
    if result.posts.is_empty() && result.users.is_empty() && result.success {
        result.success = false;
        result.error = Some(format!("channel '{channel}' not found or not public"));
    }
    result
}

pub struct ParsedPage {
    pub posts: Vec<ScrapedPost>,
    pub user: Option<ScrapedUser>,
}

/// Parse one t.me/s page into posts + channel info. Pure, testable.
pub fn parse_page(html: &str, channel: &str) -> ParsedPage {
    let doc = Html::parse_document(html);

    let post_sel = sel(".tgme_widget_message");
    let text_sel = sel(".tgme_widget_message_text");
    let date_sel = sel(".tgme_widget_message_date time");
    let link_sel = sel("a.tgme_widget_message_date");
    let views_sel = sel(".tgme_widget_message_views");
    let img_sel = sel("img");

    let mut posts = vec![];
    for el in doc.select(&post_sel) {
        let content = el
            .select(&text_sel)
            .next()
            .map(|t| t.text().collect::<String>())
            .unwrap_or_default()
            .trim()
            .to_string();

        let post_id = el
            .value()
            .attr("data-post")
            .and_then(|p| p.rsplit('/').next())
            .unwrap_or_default()
            .to_string();

        let timestamp = el
            .select(&date_sel)
            .next()
            .and_then(|t| t.value().attr("datetime"))
            .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let url = el
            .select(&link_sel)
            .next()
            .and_then(|a| a.value().attr("href"))
            .map(str::to_string);

        let views = el
            .select(&views_sel)
            .next()
            .map(|v| parse_count(&v.text().collect::<String>()))
            .unwrap_or(0);

        let media_urls: Vec<String> = el
            .select(&img_sel)
            .filter_map(|img| img.value().attr("src"))
            .filter(|src| !src.contains("emoji"))
            .map(str::to_string)
            .collect();

        if post_id.is_empty() && content.is_empty() {
            continue;
        }
        posts.push(ScrapedPost {
            platform: "telegram".into(),
            post_id,
            content,
            author_id: channel.to_string(),
            author_name: channel.to_string(),
            timestamp,
            url,
            likes: 0,
            shares: 0,
            comments: 0,
            views,
            media_urls,
            hashtags: vec![],
            mentions: vec![],
        });
    }

    let user = parse_channel_info(&doc, channel);
    ParsedPage { posts, user }
}

fn parse_channel_info(doc: &Html, channel: &str) -> Option<ScrapedUser> {
    let info = doc.select(&sel(".tgme_channel_info")).next()?;
    let display_name = info
        .select(&sel(".tgme_channel_info_header_title"))
        .next()
        .map(|t| t.text().collect::<String>().trim().to_string())
        .unwrap_or_else(|| channel.to_string());
    let bio = info
        .select(&sel(".tgme_channel_info_description"))
        .next()
        .map(|t| t.text().collect::<String>().trim().to_string())
        .unwrap_or_default();

    let mut followers = 0;
    let value_sel = sel(".counter_value");
    for counter in info.select(&sel(".tgme_channel_info_counter")) {
        let text = counter.text().collect::<String>().to_lowercase();
        if text.contains("subscriber") || text.contains("member") {
            if let Some(v) = counter.select(&value_sel).next() {
                followers = parse_count(&v.text().collect::<String>());
            }
        }
    }

    Some(ScrapedUser {
        platform: "telegram".into(),
        user_id: channel.to_string(),
        username: channel.to_string(),
        display_name,
        bio,
        profile_url: Some(format!("{BASE_URL}/{channel}")),
        avatar_url: None,
        followers,
        following: 0,
        post_count: 0,
        verified: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"
    <html><body>
      <div class="tgme_channel_info">
        <div class="tgme_channel_info_header_title">Durov's Channel</div>
        <div class="tgme_channel_info_description">Thoughts from the founder.</div>
        <div class="tgme_channel_info_counter">
          <span class="counter_value">1.2M</span> <span class="counter_type">subscribers</span>
        </div>
      </div>
      <div class="tgme_widget_message" data-post="durov/101">
        <div class="tgme_widget_message_text">Hello <b>world</b> #telegram</div>
        <img src="https://cdn.t.me/photo1.jpg">
        <img src="https://t.me/emoji/x.png">
        <span class="tgme_widget_message_views">3.5K</span>
        <a class="tgme_widget_message_date" href="https://t.me/durov/101">
          <time datetime="2024-05-01T10:30:00+00:00"></time>
        </a>
      </div>
      <div class="tgme_widget_message" data-post="durov/100">
        <div class="tgme_widget_message_text">Older post</div>
        <a class="tgme_widget_message_date" href="https://t.me/durov/100">
          <time datetime="2024-04-30T08:00:00+00:00"></time>
        </a>
      </div>
    </body></html>"#;

    #[test]
    fn parses_posts_and_channel_info() {
        let page = parse_page(FIXTURE, "durov");
        assert_eq!(page.posts.len(), 2);

        let p = &page.posts[0];
        assert_eq!(p.post_id, "101");
        assert_eq!(p.content, "Hello world #telegram");
        assert_eq!(p.views, 3500);
        assert_eq!(p.url.as_deref(), Some("https://t.me/durov/101"));
        assert_eq!(p.media_urls, vec!["https://cdn.t.me/photo1.jpg"]);
        assert_eq!(p.timestamp.to_rfc3339(), "2024-05-01T10:30:00+00:00");

        let u = page.user.expect("channel info");
        assert_eq!(u.display_name, "Durov's Channel");
        assert_eq!(u.bio, "Thoughts from the founder.");
        assert_eq!(u.followers, 1_200_000);
    }
}
