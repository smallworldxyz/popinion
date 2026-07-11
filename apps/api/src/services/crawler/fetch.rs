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
/// Cap on the response body we'll buffer. Crawl targets serve HTML pages, not
/// downloads; without this a huge or slowly-inflating body (or a redirect to
/// one) could drive per-request memory to arbitrary size.
const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;

pub async fn fetch_html(url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(user_agent())
        .build()?;
    let mut resp = client
        .get(url)
        .header("Accept-Language", "en-US,en;q=0.9")
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .send()
        .await?
        .error_for_status()?;
    // Stream with a hard cap so a chunked/undeclared body can't exhaust memory.
    let mut buf = Vec::new();
    while let Some(chunk) = resp.chunk().await? {
        if buf.len() + chunk.len() > MAX_BODY_BYTES {
            anyhow::bail!("response body exceeded {MAX_BODY_BYTES} bytes for {url}");
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}
