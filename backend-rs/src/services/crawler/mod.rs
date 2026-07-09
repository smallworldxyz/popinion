//! Social-media / web crawler subsystem.
//! Ports backend/app/services/crawler/*.py. Scrapes via CDP (LightPanda :9222,
//! Chrome fallback :9223) + HTTP, normalizing into `crate::models::CrawlResult`.
//! Fill me in: base fetch client, per-platform crawlers, results storage.
