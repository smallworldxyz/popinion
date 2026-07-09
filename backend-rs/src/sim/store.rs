use anyhow::Result;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Mutex;

/// Per-simulation social store (SQLite). Single-writer; the engine serializes
/// writes through the tokio task, reads come from the HTTP handlers.
///
/// Schema is compatible in spirit with what the frontend reads (user/post/
/// comment/follow/trace) but adds first-class `stance` and `sentiment` columns
/// so opinion analysis is a query instead of a second LLM pass.
// ponytail: one Mutex<Connection> per sim DB. A single sim is one writer; if we
// ever shard a sim across processes, move to WAL + a pool.
pub struct Store {
    conn: Mutex<Connection>,
}

pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS user (
    user_id     INTEGER PRIMARY KEY,
    agent_id    INTEGER,
    name        TEXT,
    user_name   TEXT,
    bio         TEXT DEFAULT '',
    persona     TEXT DEFAULT '',
    num_followers INTEGER DEFAULT 0,
    created_at  TEXT
);
CREATE TABLE IF NOT EXISTS post (
    post_id     INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL,
    content     TEXT NOT NULL,
    created_at  TEXT,
    round       INTEGER DEFAULT 0,
    num_likes   INTEGER DEFAULT 0,
    num_dislikes INTEGER DEFAULT 0,
    original_post_id INTEGER,
    quote_content TEXT,
    stance      TEXT,
    sentiment   REAL
);
CREATE TABLE IF NOT EXISTS comment (
    comment_id  INTEGER PRIMARY KEY AUTOINCREMENT,
    post_id     INTEGER NOT NULL,
    user_id     INTEGER NOT NULL,
    content     TEXT NOT NULL,
    created_at  TEXT,
    round       INTEGER DEFAULT 0,
    num_likes   INTEGER DEFAULT 0,
    num_dislikes INTEGER DEFAULT 0,
    stance      TEXT,
    sentiment   REAL
);
CREATE TABLE IF NOT EXISTS follow (
    follow_id   INTEGER PRIMARY KEY AUTOINCREMENT,
    follower_id INTEGER NOT NULL,
    followee_id INTEGER NOT NULL,
    created_at  TEXT
);
CREATE TABLE IF NOT EXISTS post_like (
    like_id     INTEGER PRIMARY KEY AUTOINCREMENT,
    post_id     INTEGER NOT NULL,
    user_id     INTEGER NOT NULL,
    is_dislike  INTEGER DEFAULT 0,
    created_at  TEXT
);
CREATE TABLE IF NOT EXISTS trace (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL,
    action      TEXT NOT NULL,
    info        TEXT,
    round       INTEGER DEFAULT 0,
    created_at  TEXT
);
CREATE INDEX IF NOT EXISTS idx_post_user ON post(user_id);
CREATE INDEX IF NOT EXISTS idx_comment_post ON comment(post_id);
CREATE INDEX IF NOT EXISTS idx_trace_user ON trace(user_id, action);
"#;

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Store { conn: Mutex::new(conn) })
    }

    pub fn add_user(
        &self,
        user_id: i64,
        agent_id: i64,
        name: &str,
        user_name: &str,
        bio: &str,
        persona: &str,
    ) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT OR REPLACE INTO user (user_id, agent_id, name, user_name, bio, persona, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![user_id, agent_id, name, user_name, bio, persona, now()],
        )?;
        Ok(())
    }

    pub fn add_post(
        &self,
        user_id: i64,
        content: &str,
        round: i64,
        stance: Option<&str>,
        sentiment: Option<f64>,
    ) -> Result<i64> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT INTO post (user_id, content, created_at, round, stance, sentiment)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![user_id, content, now(), round, stance, sentiment],
        )?;
        Ok(c.last_insert_rowid())
    }

    pub fn add_comment(
        &self,
        post_id: i64,
        user_id: i64,
        content: &str,
        round: i64,
        stance: Option<&str>,
        sentiment: Option<f64>,
    ) -> Result<i64> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT INTO comment (post_id, user_id, content, created_at, round, stance, sentiment)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![post_id, user_id, content, now(), round, stance, sentiment],
        )?;
        Ok(c.last_insert_rowid())
    }

    pub fn like_post(&self, post_id: i64, user_id: i64, is_dislike: bool) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT INTO post_like (post_id, user_id, is_dislike, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![post_id, user_id, is_dislike as i64, now()],
        )?;
        let col = if is_dislike { "num_dislikes" } else { "num_likes" };
        c.execute(
            &format!("UPDATE post SET {col} = {col} + 1 WHERE post_id = ?1"),
            params![post_id],
        )?;
        Ok(())
    }

    pub fn follow(&self, follower_id: i64, followee_id: i64) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT INTO follow (follower_id, followee_id, created_at) VALUES (?1, ?2, ?3)",
            params![follower_id, followee_id, now()],
        )?;
        c.execute(
            "UPDATE user SET num_followers = num_followers + 1 WHERE user_id = ?1",
            params![followee_id],
        )?;
        Ok(())
    }

    pub fn trace(&self, user_id: i64, action: &str, info: &Value, round: i64) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT INTO trace (user_id, action, info, round, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![user_id, action, info.to_string(), round, now()],
        )?;
        Ok(())
    }

    // ---- reads used by the API ----

    pub fn list_posts(&self, limit: i64, offset: i64) -> Result<Vec<Value>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT p.post_id, p.user_id, u.user_name, p.content, p.created_at, p.round,
                    p.num_likes, p.num_dislikes, p.stance, p.sentiment
             FROM post p LEFT JOIN user u ON u.user_id = p.user_id
             ORDER BY p.post_id DESC LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt
            .query_map(params![limit, offset], |r| {
                Ok(json!({
                    "post_id": r.get::<_, i64>(0)?,
                    "user_id": r.get::<_, i64>(1)?,
                    "user_name": r.get::<_, Option<String>>(2)?,
                    "content": r.get::<_, String>(3)?,
                    "created_at": r.get::<_, Option<String>>(4)?,
                    "round": r.get::<_, i64>(5)?,
                    "num_likes": r.get::<_, i64>(6)?,
                    "num_dislikes": r.get::<_, i64>(7)?,
                    "stance": r.get::<_, Option<String>>(8)?,
                    "sentiment": r.get::<_, Option<f64>>(9)?,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn count_posts(&self) -> Result<i64> {
        let c = self.conn.lock().unwrap();
        Ok(c.query_row("SELECT COUNT(*) FROM post", [], |r| r.get(0))?)
    }

    pub fn list_comments(&self, post_id: i64) -> Result<Vec<Value>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT comment_id, post_id, user_id, content, created_at, num_likes, num_dislikes, stance, sentiment
             FROM comment WHERE post_id = ?1 ORDER BY comment_id ASC",
        )?;
        let rows = stmt
            .query_map(params![post_id], |r| {
                Ok(json!({
                    "comment_id": r.get::<_, i64>(0)?,
                    "post_id": r.get::<_, i64>(1)?,
                    "user_id": r.get::<_, i64>(2)?,
                    "content": r.get::<_, String>(3)?,
                    "created_at": r.get::<_, Option<String>>(4)?,
                    "num_likes": r.get::<_, i64>(5)?,
                    "num_dislikes": r.get::<_, i64>(6)?,
                    "stance": r.get::<_, Option<String>>(7)?,
                    "sentiment": r.get::<_, Option<f64>>(8)?,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Posts + comments per round — feeds the frontend timeline view.
    pub fn timeline(&self) -> Result<Vec<Value>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT r, SUM(posts), SUM(comments), AVG(sent) FROM (
                 SELECT round r, COUNT(*) posts, 0 comments, AVG(sentiment) sent FROM post GROUP BY round
                 UNION ALL
                 SELECT round r, 0 posts, COUNT(*) comments, AVG(sentiment) sent FROM comment GROUP BY round
             ) GROUP BY r ORDER BY r ASC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(json!({
                    "round": r.get::<_, i64>(0)?,
                    "posts": r.get::<_, i64>(1)?,
                    "comments": r.get::<_, i64>(2)?,
                    "avg_sentiment": r.get::<_, Option<f64>>(3)?,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Per-agent activity summary.
    pub fn agent_stats(&self) -> Result<Vec<Value>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT u.user_id, u.user_name, u.num_followers,
                    (SELECT COUNT(*) FROM post p WHERE p.user_id = u.user_id) posts,
                    (SELECT COUNT(*) FROM comment cm WHERE cm.user_id = u.user_id) comments,
                    (SELECT AVG(sentiment) FROM post p WHERE p.user_id = u.user_id) avg_sent
             FROM user u ORDER BY posts DESC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(json!({
                    "user_id": r.get::<_, i64>(0)?,
                    "user_name": r.get::<_, Option<String>>(1)?,
                    "num_followers": r.get::<_, i64>(2)?,
                    "posts": r.get::<_, i64>(3)?,
                    "comments": r.get::<_, i64>(4)?,
                    "avg_sentiment": r.get::<_, Option<f64>>(5)?,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Recent action trace (the activity log the frontend polls).
    pub fn list_actions(&self, limit: i64) -> Result<Vec<Value>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT user_id, action, info, round, created_at FROM trace ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], |r| {
                let info: Option<String> = r.get(2)?;
                Ok(json!({
                    "user_id": r.get::<_, i64>(0)?,
                    "action": r.get::<_, String>(1)?,
                    "info": info.and_then(|s| serde_json::from_str::<Value>(&s).ok()),
                    "round": r.get::<_, i64>(3)?,
                    "created_at": r.get::<_, Option<String>>(4)?,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Aggregate stance distribution across posts — a first-class "better
    /// opinion analysis" metric that OASIS could only produce via a later pass.
    pub fn stance_distribution(&self) -> Result<Value> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT COALESCE(stance,'unknown') s, COUNT(*), AVG(sentiment)
             FROM post GROUP BY s ORDER BY COUNT(*) DESC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(json!({
                    "stance": r.get::<_, String>(0)?,
                    "count": r.get::<_, i64>(1)?,
                    "avg_sentiment": r.get::<_, Option<f64>>(2)?,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(json!(rows))
    }
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_posts_and_stance() {
        let dir = std::env::temp_dir().join(format!("popinion-test-{}", std::process::id()));
        let path = dir.join("s.db");
        let s = Store::open(&path).unwrap();
        s.add_user(1, 1, "Alice", "alice", "bio", "persona").unwrap();
        let p = s.add_post(1, "I support the policy", 0, Some("support"), Some(0.8)).unwrap();
        s.add_comment(p, 1, "agreed", 0, Some("support"), Some(0.6)).unwrap();
        s.like_post(p, 1, false).unwrap();

        assert_eq!(s.count_posts().unwrap(), 1);
        let posts = s.list_posts(10, 0).unwrap();
        assert_eq!(posts[0]["num_likes"], 1);
        assert_eq!(posts[0]["stance"], "support");
        let comments = s.list_comments(p).unwrap();
        assert_eq!(comments.len(), 1);
        let dist = s.stance_distribution().unwrap();
        assert_eq!(dist[0]["stance"], "support");
        std::fs::remove_dir_all(&dir).ok();
    }
}
