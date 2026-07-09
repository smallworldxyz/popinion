//! Persistence for crawl results: JSON files under uploads/crawl_results/.

use crate::models::CrawlResult;
use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_DIR: &str = "uploads/crawl_results";

pub fn default_dir() -> PathBuf {
    PathBuf::from(DEFAULT_DIR)
}

#[derive(Serialize)]
pub struct SavedResultInfo {
    pub filename: String,
    pub filepath: String,
    pub size_bytes: u64,
    pub created_at: String,
}

fn safe_fragment(s: &str) -> String {
    let frag: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    frag.trim_matches('_').chars().take(30).collect()
}

/// Reject anything that could escape the data dir.
pub fn valid_filename(name: &str) -> bool {
    !name.is_empty()
        && name.ends_with(".json")
        && !name.contains(['/', '\\'])
        && !name.contains("..")
}

/// Persist a crawl result as `{platform}_{hint}_{timestamp}.json`.
/// Returns the filename and full path.
pub fn save(dir: &Path, result: &CrawlResult, name_hint: &str) -> anyhow::Result<(String, PathBuf)> {
    fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let filename = format!(
        "{}_{}_{}.json",
        safe_fragment(&result.platform),
        safe_fragment(name_hint),
        Utc::now().format("%Y%m%d_%H%M%S")
    );
    let path = dir.join(&filename);
    fs::write(&path, serde_json::to_vec_pretty(result)?)
        .with_context(|| format!("write {}", path.display()))?;
    Ok((filename, path))
}

/// List saved results, newest filename first, optionally filtered by
/// platform prefix.
pub fn list(dir: &Path, platform: Option<&str>, limit: usize) -> anyhow::Result<Vec<SavedResultInfo>> {
    let mut names: Vec<String> = match fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.ends_with(".json"))
            .filter(|n| platform.is_none_or(|p| n.starts_with(p)))
            .collect(),
        Err(_) => return Ok(vec![]), // no dir yet -> nothing saved
    };
    names.sort_unstable_by(|a, b| b.cmp(a));
    names.truncate(limit);

    let mut results = vec![];
    for filename in names {
        let path = dir.join(&filename);
        let meta = fs::metadata(&path)?;
        let created: DateTime<Utc> = meta
            .created()
            .or_else(|_| meta.modified())
            .map(DateTime::from)
            .unwrap_or_else(|_| Utc::now());
        results.push(SavedResultInfo {
            filename,
            filepath: path.display().to_string(),
            size_bytes: meta.len(),
            created_at: created.to_rfc3339(),
        });
    }
    Ok(results)
}

/// Load one saved result by filename (must pass `valid_filename`).
pub fn load(dir: &Path, filename: &str) -> anyhow::Result<CrawlResult> {
    let bytes = fs::read(dir.join(filename))?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn exists(dir: &Path, filename: &str) -> bool {
    dir.join(filename).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ScrapedPost;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "popinion_crawl_test_{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample() -> CrawlResult {
        CrawlResult {
            platform: "telegram".into(),
            query: Some("durov".into()),
            posts: vec![ScrapedPost {
                platform: "telegram".into(),
                post_id: "101".into(),
                content: "hello".into(),
                author_id: "durov".into(),
                author_name: "durov".into(),
                timestamp: Utc::now(),
                url: None,
                likes: 1,
                shares: 2,
                comments: 3,
                views: 4,
                media_urls: vec![],
                hashtags: vec!["x".into()],
                mentions: vec![],
            }],
            users: vec![],
            crawled_at: Utc::now(),
            success: true,
            error: None,
        }
    }

    #[test]
    fn save_list_load_round_trip() {
        let dir = temp_dir();
        let (filename, path) = save(&dir, &sample(), "durov").unwrap();
        assert!(path.is_file());
        assert!(filename.starts_with("telegram_durov_"));

        let listed = list(&dir, None, 20).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].filename, filename);
        assert!(listed[0].size_bytes > 0);

        // platform filter
        assert_eq!(list(&dir, Some("twitter"), 20).unwrap().len(), 0);
        assert_eq!(list(&dir, Some("telegram"), 20).unwrap().len(), 1);

        let loaded = load(&dir, &filename).unwrap();
        assert_eq!(loaded.platform, "telegram");
        assert_eq!(loaded.posts.len(), 1);
        assert_eq!(loaded.posts[0].hashtags, vec!["x"]);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn filename_validation_blocks_traversal() {
        assert!(valid_filename("telegram_durov_20240101_000000.json"));
        assert!(!valid_filename("../secrets.json"));
        assert!(!valid_filename("a/b.json"));
        assert!(!valid_filename("a\\b.json"));
        assert!(!valid_filename("notjson.txt"));
        assert!(!valid_filename(""));
    }
}
