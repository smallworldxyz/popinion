//! Social-media / web crawler subsystem.
//! Ports backend/app/services/crawler/*.py. Fetches over plain HTTP (see
//! fetch.rs for the CDP swap point), parses into `crate::models::CrawlResult`,
//! persists JSON under uploads/crawl_results/, and bridges scraped data into
//! OASIS simulation agent profiles.

pub mod bridge;
pub mod facebook;
pub mod fetch;
pub mod storage;
pub mod telegram;
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
    fn extract_tagged_finds_hashtags_and_mentions() {
        let c = "big #AI news, thanks @sam_a! see #ml2024.";
        assert_eq!(extract_tagged(c, '#'), vec!["AI", "ml2024"]);
        assert_eq!(extract_tagged(c, '@'), vec!["sam_a"]);
    }
}
