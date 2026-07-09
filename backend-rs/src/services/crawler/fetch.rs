//! HTTP fetch layer for the crawlers.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

const USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
];

fn user_agent() -> &'static str {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize)
        .unwrap_or(0);
    USER_AGENTS[n % USER_AGENTS.len()]
}

// ponytail: plain HTTP GET only — the ceiling. Telegram's t.me/s pages are
// server-rendered and work fine here, but X.com and m.facebook.com serve a JS
// shell to non-browser clients, so their parsers find zero posts and the
// crawlers report success=false with a clear error. To lift the ceiling, swap
// this function to drive the CDP browsers from docker-compose (LightPanda
// :9222, Chrome fallback :9223) via e.g. `chromiumoxide` and return the
// rendered DOM instead.
pub async fn fetch_html(url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(user_agent())
        .build()?;
    let resp = client
        .get(url)
        .header("Accept-Language", "en-US,en;q=0.9")
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .send()
        .await?
        .error_for_status()?;
    Ok(resp.text().await?)
}
