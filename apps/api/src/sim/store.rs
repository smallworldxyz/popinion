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
    created_at  TEXT,
    UNIQUE(follower_id, followee_id)
);
CREATE TABLE IF NOT EXISTS post_like (
    like_id     INTEGER PRIMARY KEY AUTOINCREMENT,
    post_id     INTEGER NOT NULL,
    user_id     INTEGER NOT NULL,
    is_dislike  INTEGER DEFAULT 0,
    created_at  TEXT,
    UNIQUE(post_id, user_id)
);
CREATE TABLE IF NOT EXISTS trace (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL,
    action      TEXT NOT NULL,
    info        TEXT,
    round       INTEGER DEFAULT 0,
    created_at  TEXT
);
CREATE TABLE IF NOT EXISTS independent_stance (
    kind        TEXT NOT NULL,
    ref_id      INTEGER NOT NULL,
    user_id     INTEGER,
    self_stance TEXT,
    ind_stance  TEXT,
    created_at  TEXT,
    PRIMARY KEY (kind, ref_id)
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
        // busy_timeout lets a concurrent writer (e.g. /classify-stance writing
        // labels while the engine is still running) wait for the lock instead of
        // failing immediately with SQLITE_BUSY.
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=5000;")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Store { conn: Mutex::new(conn) })
    }

    pub fn add_user(
        &self,
        user_id: i64,
        name: &str,
        user_name: &str,
        bio: &str,
        persona: &str,
    ) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT OR REPLACE INTO user (user_id, name, user_name, bio, persona, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![user_id, name, user_name, bio, persona, now()],
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
        // One reaction per (post, user): a repeat like/dislike is ignored so a
        // single agent can't inflate num_likes across the run's many rounds.
        let inserted = c.execute(
            "INSERT OR IGNORE INTO post_like (post_id, user_id, is_dislike, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![post_id, user_id, is_dislike as i64, now()],
        )?;
        if inserted > 0 {
            let col = if is_dislike { "num_dislikes" } else { "num_likes" };
            c.execute(
                &format!("UPDATE post SET {col} = {col} + 1 WHERE post_id = ?1"),
                params![post_id],
            )?;
        }
        Ok(())
    }

    pub fn follow(&self, follower_id: i64, followee_id: i64) -> Result<()> {
        let c = self.conn.lock().unwrap();
        // One edge per (follower, followee): a repeat follow is ignored so
        // num_followers stays a distinct count and re-seeding can't double it.
        let inserted = c.execute(
            "INSERT OR IGNORE INTO follow (follower_id, followee_id, created_at) VALUES (?1, ?2, ?3)",
            params![follower_id, followee_id, now()],
        )?;
        if inserted > 0 {
            c.execute(
                "UPDATE user SET num_followers = num_followers + 1 WHERE user_id = ?1",
                params![followee_id],
            )?;
        }
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

    /// A personalized feed for one agent: posts from accounts it follows first
    /// (the network), then globally trending by likes, then recent — excluding
    /// the agent's own posts. This is what makes Follow and likes matter:
    /// influence and engagement shape what each agent sees, enabling echo
    /// chambers and cascades instead of one shared global timeline.
    pub fn feed_for(&self, user_id: i64, limit: i64) -> Result<Vec<Value>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT p.post_id, p.user_id, u.user_name, p.content, p.num_likes, p.num_dislikes,
                    p.stance,
                    (SELECT COUNT(*) FROM follow f
                     WHERE f.follower_id = ?1 AND f.followee_id = p.user_id) AS followed
             FROM post p LEFT JOIN user u ON u.user_id = p.user_id
             WHERE p.user_id != ?1
             ORDER BY followed DESC, p.num_likes DESC, p.post_id DESC
             LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![user_id, limit], |r| {
                Ok(json!({
                    "post_id": r.get::<_, i64>(0)?,
                    "user_id": r.get::<_, i64>(1)?,
                    "user_name": r.get::<_, Option<String>>(2)?,
                    "content": r.get::<_, String>(3)?,
                    "num_likes": r.get::<_, i64>(4)?,
                    "num_dislikes": r.get::<_, i64>(5)?,
                    "stance": r.get::<_, Option<String>>(6)?,
                    "followed": r.get::<_, i64>(7)? > 0,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// An agent's own most-recent posts — context for in-character interviews
    /// so answers reflect what the agent actually said in the simulation.
    pub fn posts_by_user(&self, user_id: i64, limit: i64) -> Result<Vec<Value>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT post_id, content, round, stance, sentiment
             FROM post WHERE user_id = ?1 ORDER BY post_id DESC LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![user_id, limit], |r| {
                Ok(json!({
                    "post_id": r.get::<_, i64>(0)?,
                    "content": r.get::<_, String>(1)?,
                    "round": r.get::<_, i64>(2)?,
                    "stance": r.get::<_, Option<String>>(3)?,
                    "sentiment": r.get::<_, Option<f64>>(4)?,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// An agent's own most-recent comments — interview context alongside posts,
    /// so agents who only commented aren't treated as silent.
    pub fn comments_by_user(&self, user_id: i64, limit: i64) -> Result<Vec<Value>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT comment_id, post_id, content, round, stance
             FROM comment WHERE user_id = ?1 ORDER BY comment_id DESC LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![user_id, limit], |r| {
                Ok(json!({
                    "comment_id": r.get::<_, i64>(0)?,
                    "post_id": r.get::<_, i64>(1)?,
                    "content": r.get::<_, String>(2)?,
                    "round": r.get::<_, i64>(3)?,
                    "stance": r.get::<_, Option<String>>(4)?,
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
            // Count-weighted mean sentiment: sum every non-null sentiment across
            // posts+comments and divide by their count. Averaging the two
            // per-source AVGs (the old form) ignored row counts and skewed the curve.
            "SELECT r, SUM(posts), SUM(comments), SUM(sent_sum) / NULLIF(SUM(sent_cnt), 0) FROM (
                 SELECT round r, COUNT(*) posts, 0 comments, COUNT(sentiment) sent_cnt, COALESCE(SUM(sentiment), 0.0) sent_sum FROM post GROUP BY round
                 UNION ALL
                 SELECT round r, 0 posts, COUNT(*) comments, COUNT(sentiment) sent_cnt, COALESCE(SUM(sentiment), 0.0) sent_sum FROM comment GROUP BY round
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

    /// Recent action trace (the activity log the frontend polls). Exposure rows
    /// are instrumentation (who was served what), not agent actions — they would
    /// flood the feed at one row per active agent per round.
    pub fn list_actions(&self, limit: i64) -> Result<Vec<Value>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT user_id, action, info, round, created_at FROM trace
             WHERE action != 'exposed' ORDER BY id DESC LIMIT ?1",
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

    /// All stance-bearing acts (posts + comments, seed excluded) — the raw rows
    /// for sim::stance math (agent distribution, exposure shift).
    pub fn stance_acts(&self) -> Result<Vec<super::stance::StanceAct>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT user_id, stance, round, created_at FROM post
             WHERE stance IS NOT NULL AND stance != 'seed'
             UNION ALL
             SELECT user_id, stance, round, created_at FROM comment
             WHERE stance IS NOT NULL AND stance != 'seed'",
        )?;
        let acts = stmt
            .query_map([], |r| {
                Ok(super::stance::StanceAct {
                    user_id: r.get(0)?,
                    stance: r.get(1)?,
                    round: r.get(2)?,
                    created_at: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(acts)
    }

    /// Agent-weighted stance distribution: one vote per agent, their most recent
    /// stance-bearing action (post or comment). This is the honest population
    /// metric — post-weighted counts let one hyperactive agent dominate.
    /// The store only reads the raw acts; the math lives in sim::stance.
    pub fn agent_stance_distribution(&self) -> Result<Value> {
        Ok(super::stance::agent_distribution(&self.stance_acts()?))
    }

    /// Exposure rows the engine writes each round: (agent, round, served post
    /// ids). Parsed in Rust — the volume is bounded (one row per active agent
    /// per round), so no SQL JSON support is needed.
    pub fn exposures(&self) -> Result<Vec<(i64, i64, Vec<i64>)>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT user_id, round, info FROM trace WHERE action = 'exposed'")?;
        let rows = stmt
            .query_map([], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, Option<String>>(2)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows
            .into_iter()
            .map(|(uid, round, info)| {
                let ids = info
                    .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                    .and_then(|v| {
                        v["post_ids"]
                            .as_array()
                            .map(|a| a.iter().filter_map(|x| x.as_i64()).collect())
                    })
                    .unwrap_or_default();
                (uid, round, ids)
            })
            .collect())
    }

    /// The seed/injected posts (or one specific post) with engagement counts —
    /// the subjects of the /spread readout.
    pub fn spread_posts(&self, post_id: Option<i64>) -> Result<Vec<Value>> {
        let c = self.conn.lock().unwrap();
        let base = "SELECT p.post_id, p.user_id, p.content, p.round, p.num_likes, p.num_dislikes,
                    (SELECT COUNT(*) FROM comment cm WHERE cm.post_id = p.post_id) AS num_comments
             FROM post p";
        let map = |r: &rusqlite::Row| -> rusqlite::Result<Value> {
            Ok(json!({
                "post_id": r.get::<_, i64>(0)?,
                "user_id": r.get::<_, i64>(1)?,
                "content": r.get::<_, String>(2)?,
                "round": r.get::<_, i64>(3)?,
                "num_likes": r.get::<_, i64>(4)?,
                "num_dislikes": r.get::<_, i64>(5)?,
                "num_comments": r.get::<_, i64>(6)?,
            }))
        };
        let (sql, args) = match post_id {
            Some(pid) => (format!("{base} WHERE p.post_id = ?1"), vec![pid]),
            None => (format!("{base} WHERE p.stance = 'seed' ORDER BY p.post_id ASC"), vec![]),
        };
        let mut stmt = c.prepare(&sql)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(args), map)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Aggregate stance distribution across posts — a first-class "better
    /// opinion analysis" metric that OASIS could only produce via a later pass.
    pub fn stance_distribution(&self) -> Result<Value> {
        let c = self.conn.lock().unwrap();
        // Exclude seed posts: 'seed' marks the injected event, not an opinion.
        // Counting it inflates every run's shared mass and biases the honesty
        // metrics toward "no effect".
        let mut stmt = c.prepare(
            "SELECT COALESCE(stance,'unknown') s, COUNT(*), AVG(sentiment)
             FROM post WHERE stance IS NULL OR stance != 'seed'
             GROUP BY s ORDER BY COUNT(*) DESC",
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

    // ---- independent stance measurement (monoculture check) ----

    /// Posts/comments not yet independently labelled (seed excluded), for the
    /// classifier to process. Returns kind, ref_id, user_id, content, self_stance.
    pub fn unlabeled_content(&self, limit: i64) -> Result<Vec<Value>> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare(
            "SELECT 'post' AS kind, p.post_id AS ref_id, p.user_id, p.content, p.stance
             FROM post p
             WHERE (p.stance IS NULL OR p.stance != 'seed')
               AND NOT EXISTS (SELECT 1 FROM independent_stance s WHERE s.kind='post' AND s.ref_id=p.post_id)
             UNION ALL
             SELECT 'comment', cm.comment_id, cm.user_id, cm.content, cm.stance
             FROM comment cm
             WHERE (cm.stance IS NULL OR cm.stance != 'seed')
               AND NOT EXISTS (SELECT 1 FROM independent_stance s WHERE s.kind='comment' AND s.ref_id=cm.comment_id)
             LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], |r| {
                Ok(json!({
                    "kind": r.get::<_, String>(0)?,
                    "ref_id": r.get::<_, i64>(1)?,
                    "user_id": r.get::<_, i64>(2)?,
                    "content": r.get::<_, String>(3)?,
                    "self_stance": r.get::<_, Option<String>>(4)?,
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn set_independent_stance(
        &self,
        kind: &str,
        ref_id: i64,
        user_id: i64,
        self_stance: Option<&str>,
        ind_stance: &str,
    ) -> Result<()> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT OR REPLACE INTO independent_stance (kind, ref_id, user_id, self_stance, ind_stance, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![kind, ref_id, user_id, self_stance, ind_stance, now()],
        )?;
        Ok(())
    }

    /// How often the agents' self-reported stance matched the independent label.
    /// Low agreement means the self-reports (and headline numbers) are unreliable.
    /// The store only reads the raw labels; the math lives in sim::stance.
    pub fn stance_agreement(&self) -> Result<Value> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT self_stance, ind_stance FROM independent_stance")?;
        let labels = stmt
            .query_map([], |r| {
                Ok(super::stance::IndependentLabel {
                    self_stance: r.get(0)?,
                    ind_stance: r.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(super::stance::agreement(&labels))
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
        s.add_user(1, "Alice", "alice", "bio", "persona").unwrap();
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

    #[test]
    fn follow_and_like_are_idempotent() {
        let dir = std::env::temp_dir().join(format!("popinion-idem-{}", std::process::id()));
        let s = Store::open(&dir.join("s.db")).unwrap();
        s.add_user(1, "A", "a", "", "").unwrap();
        s.add_user(2, "B", "b", "", "").unwrap();
        let p = s.add_post(2, "hi", 0, None, None).unwrap();

        // Repeated follow / like from the same agent must not inflate counts.
        s.follow(1, 2).unwrap();
        s.follow(1, 2).unwrap();
        s.like_post(p, 1, false).unwrap();
        s.like_post(p, 1, false).unwrap();

        let followers = s.agent_stats().unwrap();
        let b = followers.iter().find(|u| u["user_id"] == 2).unwrap();
        assert_eq!(b["num_followers"], 1, "duplicate follow must not double the count");
        assert_eq!(s.list_posts(10, 0).unwrap()[0]["num_likes"], 1, "duplicate like must not double the count");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn independent_labels_drive_distribution_and_agreement() {
        let dir = std::env::temp_dir().join(format!("popinion-indep-{}", std::process::id()));
        let s = Store::open(&dir.join("s.db")).unwrap();
        s.add_user(1, "u", "u", "", "").unwrap();
        let p1 = s.add_post(1, "I back the plan", 0, Some("support"), None).unwrap();
        let p2 = s.add_post(1, "actually this is terrible", 0, Some("support"), None).unwrap();
        // Independent classifier agrees on p1, disagrees on p2 (self=support, indep=oppose).
        s.set_independent_stance("post", p1, 1, Some("support"), "support").unwrap();
        s.set_independent_stance("post", p2, 1, Some("support"), "oppose").unwrap();

        let ag = s.stance_agreement().unwrap();
        assert_eq!(ag["labelled"], 2);
        assert_eq!(ag["agree"], 1);
        assert!((ag["agreement_rate"].as_f64().unwrap() - 0.5).abs() < 1e-9);
        // Independent marginal (same labelled set): support 1, oppose 1.
        let counts: std::collections::HashMap<String, i64> = ag["independent_distribution"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| (r["stance"].as_str().unwrap().to_string(), r["count"].as_i64().unwrap()))
            .collect();
        assert_eq!(counts.get("support"), Some(&1));
        assert_eq!(counts.get("oppose"), Some(&1));

        // Already-labelled posts are not re-served to the classifier.
        assert!(s.unlabeled_content(100).unwrap().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn feed_prioritizes_followed_then_trending_and_excludes_self() {
        let dir = std::env::temp_dir().join(format!("popinion-feed-{}", std::process::id()));
        let s = Store::open(&dir.join("s.db")).unwrap();
        for id in 1..=3 {
            s.add_user(id, "u", "u", "", "").unwrap();
        }
        let p_followee = s.add_post(2, "from someone I follow", 0, Some("support"), None).unwrap();
        let p_trending = s.add_post(3, "popular but not followed", 0, Some("oppose"), None).unwrap();
        s.add_post(1, "my own post", 0, Some("support"), None).unwrap();
        // Trending post has more likes, but agent 1 only follows agent 2.
        s.like_post(p_trending, 1, false).unwrap();
        s.like_post(p_trending, 2, false).unwrap();
        s.follow(1, 2).unwrap();

        let feed = s.feed_for(1, 10).unwrap();
        assert_eq!(feed[0]["post_id"], p_followee, "followed post ranks first despite fewer likes");
        assert_eq!(feed[0]["followed"], true);
        assert_eq!(feed[1]["post_id"], p_trending, "then trending by likes");
        assert!(feed.iter().all(|p| p["user_id"] != 1), "own posts excluded");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn exposure_rows_and_spread_posts_roundtrip() {
        let dir = std::env::temp_dir().join(format!("popinion-spread-{}", std::process::id()));
        let s = Store::open(&dir.join("s.db")).unwrap();
        s.add_user(1, "A", "a", "", "").unwrap();
        s.add_user(2, "B", "b", "", "").unwrap();
        let seed = s.add_post(-1, "the event", 0, Some("seed"), None).unwrap();
        let normal = s.add_post(1, "a reaction", 1, Some("support"), None).unwrap();
        s.add_comment(seed, 2, "reply on the event", 1, Some("oppose"), None).unwrap();
        s.trace(1, "exposed", &json!({ "post_ids": [seed, normal] }), 1).unwrap();
        s.trace(2, "exposed", &json!({ "post_ids": [normal] }), 2).unwrap();

        let ex = s.exposures().unwrap();
        assert_eq!(ex.len(), 2);
        assert!(ex.contains(&(1, 1, vec![seed, normal])));
        assert!(ex.contains(&(2, 2, vec![normal])));

        // Default scope: only seed/injected posts, with engagement counts.
        let sp = s.spread_posts(None).unwrap();
        assert_eq!(sp.len(), 1);
        assert_eq!(sp[0]["post_id"], seed);
        assert_eq!(sp[0]["num_comments"], 1);
        // Explicit post_id: any post is inspectable.
        let sp2 = s.spread_posts(Some(normal)).unwrap();
        assert_eq!(sp2[0]["post_id"], normal);

        // Exposure rows are instrumentation — kept out of the activity log.
        let actions = s.list_actions(50).unwrap();
        assert!(actions.iter().all(|a| a["action"] != "exposed"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn agent_weighted_counts_one_vote_per_agent() {
        let dir = std::env::temp_dir().join(format!("popinion-agentw-{}", std::process::id()));
        let s = Store::open(&dir.join("s.db")).unwrap();
        s.add_user(1, "A", "a", "", "").unwrap();
        s.add_user(2, "B", "b", "", "").unwrap();
        // Agent 1 is hyperactive: 3 support posts. Agent 2: one oppose comment.
        for _ in 0..3 {
            s.add_post(1, "yes", 0, Some("support"), Some(0.5)).unwrap();
        }
        let p = s.add_post(2, "topic", 0, Some("seed"), None).unwrap();
        s.add_comment(p, 2, "no", 1, Some("oppose"), Some(-0.5)).unwrap();

        // Post-weighted: support dominates 3:0 (seed excluded).
        let post_w = s.stance_distribution().unwrap();
        assert_eq!(post_w[0]["stance"], "support");
        assert_eq!(post_w[0]["count"], 3);

        // Agent-weighted: one vote each → support 1, oppose 1.
        let agent_w = s.agent_stance_distribution().unwrap();
        let counts: std::collections::HashMap<String, i64> = agent_w
            .as_array()
            .unwrap()
            .iter()
            .map(|r| (r["stance"].as_str().unwrap().to_string(), r["count"].as_i64().unwrap()))
            .collect();
        assert_eq!(counts.get("support"), Some(&1));
        assert_eq!(counts.get("oppose"), Some(&1));
        std::fs::remove_dir_all(&dir).ok();
    }
}
