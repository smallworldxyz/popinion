//! Social-media / web crawler subsystem.
//! Fetches over plain HTTP (see fetch.rs for the CDP swap point), parses into
//! `crate::models::CrawlResult`, persists JSON under uploads/crawl_results/,
//! and bridges scraped data into simulation agent profiles.

pub mod bridge;
pub mod facebook;
pub mod fetch;
pub mod google_trends;
pub mod reddit;
pub mod storage;
pub mod telegram;
pub mod threads;
pub mod tiktok;
pub mod twitter;

/// Parse a count string like "1.2K", "3.5M" or "1,234" into an integer.
pub fn parse_count(text: &str) -> i64 {
    let text = text.trim().to_uppercase();
    if text.is_empty() {
        return 0;
    }
    let mut chars = text
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .peekable();
    let mut num = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() || c == '.' {
            num.push(c);
            chars.next();
        } else if c == ',' {
            chars.next();
        } else {
            break;
        }
    }
    // Multiplier suffix only counts when it directly follows the number.
    let mult = match chars.peek() {
        Some('K') => 1_000f64,
        Some('M') => 1_000_000f64,
        Some('B') => 1_000_000_000f64,
        _ => 1f64,
    };
    num.parse::<f64>().map(|n| (n * mult) as i64).unwrap_or(0)
}

/// Flatten a crawl result into one plain-text document for the graph-build
/// corpus. Same shape as an uploaded file's extracted text, so it flows
/// through the identical chunk → extract → upsert pipeline (provenance and
/// evidence tracking included). Posts without text (media-only) are skipped.
pub fn corpus_document(result: &crate::models::CrawlResult) -> String {
    let channel = result.query.as_deref().unwrap_or("unknown");
    // Says what this material IS. A channel broadcasts; it does not answer
    // back. Reading these posts as public reaction would put the channel's own
    // position in the mouth of an audience that never spoke here.
    let mut doc = format!(
        "Posts BROADCAST BY the {} channel @{channel}. These are the channel's own \
         statements, not replies from its audience.\n",
        result.platform
    );
    if let Some(user) = result.users.first() {
        if !user.display_name.is_empty() {
            doc.push_str(&format!("Channel: {}", user.display_name));
            if !user.bio.is_empty() {
                doc.push_str(&format!(" — {}", user.bio));
            }
            if user.followers > 0 {
                doc.push_str(&format!(" ({} subscribers)", user.followers));
            }
            doc.push('\n');
        }
    }
    doc.push('\n');
    let posts = result
        .posts
        .iter()
        .filter(|p| !p.content.trim().is_empty())
        .map(|p| format!("[{}] {}", p.timestamp.format("%Y-%m-%d"), p.content.trim()))
        .collect::<Vec<_>>()
        .join("\n\n");
    doc.push_str(&posts);
    doc
}

/// Extract `#hashtags` or `@mentions` (prefix-tagged words) from content.
pub fn extract_tagged(content: &str, prefix: char) -> Vec<String> {
    content
        .split_whitespace()
        .filter_map(|w| w.strip_prefix(prefix))
        .map(|w| {
            w.chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The corpus must say what the material is. A channel broadcasts and its
    /// audience does not reply here, so anything downstream that treats these
    /// posts as public reaction is attributing the channel's own position to
    /// people who never spoke.
    #[test]
    fn the_corpus_labels_channel_posts_as_broadcasts() {
        // Built from JSON: these models have no Default and do not need one.
        let result: crate::models::CrawlResult = serde_json::from_value(serde_json::json!({
            "platform": "telegram",
            "query": "examplechannel",
            "users": [{
                "platform": "telegram", "user_id": "examplechannel",
                "display_name": "Example News", "followers": 12000
            }],
            "posts": [{
                "platform": "telegram", "post_id": "1", "author_id": "examplechannel",
                "content": "We announced the policy today."
            }]
        }))
        .unwrap();
        let doc = corpus_document(&result);
        assert!(doc.contains("BROADCAST BY"), "names what the material is");
        assert!(doc.contains("not replies from its audience"), "and what it is not");
        assert!(doc.contains("12000 subscribers"), "reach is kept, not discarded");
        assert!(doc.contains("We announced the policy today."));
    }

    #[test]
    fn parse_count_handles_suffixes_and_plain() {
        assert_eq!(parse_count("1.2K"), 1200);
        assert_eq!(parse_count("3.5M"), 3_500_000);
        assert_eq!(parse_count("1,234"), 1234);
        assert_eq!(parse_count("42 subscribers"), 42);
        assert_eq!(parse_count("2B"), 2_000_000_000);
        assert_eq!(parse_count("1.2K posts"), 1200);
        assert_eq!(parse_count(""), 0);
        assert_eq!(parse_count("n/a"), 0);
    }

    #[test]
    fn corpus_document_flattens_posts_and_skips_media_only() {
        use crate::models::{CrawlResult, ScrapedPost, ScrapedUser};
        let post = |id: &str, content: &str| ScrapedPost {
            platform: "telegram".into(),
            post_id: id.into(),
            content: content.into(),
            author_id: "chan".into(),
            author_name: "chan".into(),
            timestamp: chrono::DateTime::parse_from_rfc3339("2024-05-01T10:30:00+00:00")
                .unwrap()
                .with_timezone(&chrono::Utc),
            url: None,
            likes: 0,
            shares: 0,
            comments: 0,
            views: 0,
            media_urls: vec![],
            hashtags: vec![],
            mentions: vec![],
        };
        let result = CrawlResult {
            platform: "telegram".into(),
            query: Some("chan".into()),
            posts: vec![post("1", "Fuel subsidy ends next year"), post("2", "  ")],
            users: vec![ScrapedUser {
                platform: "telegram".into(),
                user_id: "chan".into(),
                username: "chan".into(),
                display_name: "The Channel".into(),
                bio: "News and views".into(),
                profile_url: None,
                avatar_url: None,
                followers: 0,
                following: 0,
                post_count: 0,
                verified: false,
            }],
            crawled_at: chrono::Utc::now(),
            success: true,
            error: None,
        };
        let doc = corpus_document(&result);
        assert!(doc.contains("@chan"));
        assert!(doc.contains("The Channel — News and views"));
        assert!(doc.contains("[2024-05-01] Fuel subsidy ends next year"));
        // The empty (media-only) post contributes nothing.
        assert!(!doc.contains("[2024-05-01] \n"));
        assert!(!doc.trim_end().ends_with("[2024-05-01]"));
    }

    #[test]
    fn extract_tagged_finds_hashtags_and_mentions() {
        let c = "big #AI news, thanks @sam_a! see #ml2024.";
        assert_eq!(extract_tagged(c, '#'), vec!["AI", "ml2024"]);
        assert_eq!(extract_tagged(c, '@'), vec!["sam_a"]);
    }
}
