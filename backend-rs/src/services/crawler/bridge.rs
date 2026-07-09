//! Bridge scraped real-world data into OASIS simulation agent profiles.
//! Ports backend/app/services/pubop_bridge.py (+ the OasisAgentProfile shape
//! from oasis_profile_generator.py). Inference is deterministic (keyed off
//! user_id / bio keywords) instead of random, so seeds are reproducible.

use crate::models::{CrawlResult, ScrapedPost, ScrapedUser};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

const MBTI_TYPES: [&str; 16] = [
    "INTJ", "INTP", "ENTJ", "ENTP", "INFJ", "INFP", "ENFJ", "ENFP", "ISTJ", "ISFJ", "ESTJ",
    "ESFJ", "ISTP", "ISFP", "ESTP", "ESFP",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OasisAgentProfile {
    pub user_id: i64,
    pub user_name: String,
    pub name: String,
    pub bio: String,
    pub persona: String,
    pub karma: i64,
    pub friend_count: i64,
    pub follower_count: i64,
    pub statuses_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mbti: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profession: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub interested_topics: Vec<String>,
    pub source_entity_uuid: Option<String>,
    pub source_entity_type: Option<String>,
    pub created_at: String,
}

/// Real-world data packaged for simulation seeding. Unlike the Python
/// `to_dict()` (counts only), this serializes the full profiles and initial
/// posts so the caller actually gets the data.
#[derive(Serialize)]
pub struct RealDataSeed {
    pub platform: String,
    pub query: Option<String>,
    pub crawled_at: String,
    pub profiles: Vec<OasisAgentProfile>,
    pub initial_posts: Vec<Value>,
    pub trending_topics: Vec<String>,
    pub original_post_count: usize,
    pub original_user_count: usize,
}

/// Convert a crawl result into a simulation seed.
pub fn create_seed(result: &CrawlResult, anonymize: bool, max_profiles: usize) -> RealDataSeed {
    const MAX_POSTS: usize = 500;

    let mut profiles: Vec<OasisAgentProfile> = result
        .users
        .iter()
        .take(max_profiles)
        .enumerate()
        .map(|(i, u)| user_to_profile(u, i as i64, anonymize))
        .collect();

    // Fill remaining profile slots from post authors not already covered.
    let mut seen: HashSet<String> = profiles
        .iter()
        .filter_map(|p| p.source_entity_uuid.clone())
        .collect();
    for post in &result.posts {
        if profiles.len() >= max_profiles {
            break;
        }
        if !seen.insert(post.author_id.clone()) {
            continue;
        }
        let id = profiles.len() as i64;
        profiles.push(OasisAgentProfile {
            user_id: id,
            user_name: if anonymize {
                format!("user_{id:05}")
            } else {
                sanitize_username(&post.author_name)
            },
            name: if anonymize {
                format!("User {id}")
            } else {
                post.author_name.clone()
            },
            bio: String::new(),
            persona: format!("Active {} user who posts about various topics.", post.platform),
            karma: 1000,
            friend_count: 100,
            follower_count: 150,
            statuses_count: 500,
            age: None,
            mbti: Some(MBTI_TYPES[id as usize % 16].to_string()),
            profession: None,
            interested_topics: vec![],
            source_entity_uuid: Some(post.author_id.clone()),
            source_entity_type: Some(format!("{}_user", post.platform)),
            created_at: Utc::now().format("%Y-%m-%d").to_string(),
        });
    }

    let initial_posts = posts_to_initial_posts(&result.posts[..result.posts.len().min(MAX_POSTS)], &profiles);

    RealDataSeed {
        platform: result.platform.clone(),
        query: result.query.clone(),
        crawled_at: result.crawled_at.to_rfc3339(),
        profiles,
        initial_posts,
        trending_topics: vec![],
        original_post_count: result.posts.len(),
        original_user_count: result.users.len(),
    }
}

fn user_to_profile(user: &ScrapedUser, user_id: i64, anonymize: bool) -> OasisAgentProfile {
    let (user_name, name) = if anonymize {
        (format!("user_{user_id:05}"), format!("User {user_id}"))
    } else {
        let uname = sanitize_username(&user.username);
        let display = if user.display_name.is_empty() {
            uname.clone()
        } else {
            user.display_name.clone()
        };
        (uname, display)
    };

    OasisAgentProfile {
        user_id,
        user_name,
        name,
        bio: user.bio.chars().take(500).collect(),
        persona: persona_from_bio(&user.bio, &user.platform),
        karma: 1000,
        friend_count: user.following,
        follower_count: user.followers,
        statuses_count: user.post_count,
        age: infer_age(&user.bio, user_id),
        mbti: Some(MBTI_TYPES[user_id as usize % 16].to_string()),
        profession: infer_profession(&user.bio),
        interested_topics: extract_topics(&user.bio),
        source_entity_uuid: Some(user.user_id.clone()),
        source_entity_type: Some(format!("{}_user", user.platform)),
        created_at: Utc::now().format("%Y-%m-%d").to_string(),
    }
}

fn posts_to_initial_posts(posts: &[ScrapedPost], profiles: &[OasisAgentProfile]) -> Vec<Value> {
    let by_author: HashMap<&str, &OasisAgentProfile> = profiles
        .iter()
        .filter_map(|p| p.source_entity_uuid.as_deref().map(|id| (id, p)))
        .collect();

    posts
        .iter()
        .filter_map(|post| {
            let profile = by_author.get(post.author_id.as_str())?;
            let mut v = json!({
                "user_id": profile.user_id,
                "username": profile.user_name,
                "content": sanitize_content(&post.content),
                "created_at": post.timestamp.to_rfc3339(),
                "likes": post.likes,
                "shares": post.shares,
                "comments": post.comments,
                "platform": post.platform,
            });
            if !post.hashtags.is_empty() {
                v["hashtags"] = json!(post.hashtags);
            }
            Some(v)
        })
        .collect()
}

fn persona_from_bio(bio: &str, platform: &str) -> String {
    let clean: String = bio.trim().chars().take(300).collect();
    if clean.is_empty() {
        return format!("An active {platform} user who engages with content on various topics.");
    }
    format!(
        "A real {platform} user. Profile description: {clean}. \
         Behaves authentically based on their observed online patterns."
    )
}

fn infer_age(bio: &str, seed: i64) -> Option<i64> {
    if bio.is_empty() {
        return None;
    }
    let bio = bio.to_lowercase();

    // Explicit "NN yo" / "NN years old" style mentions.
    let tokens: Vec<&str> = bio.split_whitespace().collect();
    for pair in tokens.windows(2) {
        if let Ok(age) = pair[0].parse::<i64>() {
            if (13..=99).contains(&age) && matches!(pair[1].trim_matches('.'), "yo" | "y.o" | "years" | "year") {
                return Some(age);
            }
        }
    }

    let pick = |lo: i64, hi: i64| Some(lo + seed.rem_euclid(hi - lo + 1));
    if ["student", "university", "college", "grad"].iter().any(|w| bio.contains(w)) {
        return pick(18, 25);
    }
    if ["dad", "mom", "father", "mother", "parent"].iter().any(|w| bio.contains(w)) {
        return pick(30, 45);
    }
    if ["retired", "grandpa", "grandma"].iter().any(|w| bio.contains(w)) {
        return pick(60, 75);
    }
    None
}

fn infer_profession(bio: &str) -> Option<String> {
    let bio = bio.to_lowercase();
    let table: [(&str, &[&str]); 8] = [
        ("developer", &["developer", "programmer", "coder", "software", "engineer"]),
        ("designer", &["designer", "artist", "creative", "illustrator"]),
        ("writer", &["writer", "author", "journalist", "editor", "blogger"]),
        ("teacher", &["teacher", "professor", "educator", "instructor"]),
        ("entrepreneur", &["entrepreneur", "founder", "ceo", "startup"]),
        ("student", &["student", "studying", "university", "college"]),
        ("healthcare", &["doctor", "nurse", "medical", "healthcare"]),
        ("marketing", &["marketing", "social media", "content creator"]),
    ];
    table
        .iter()
        .find(|(_, kws)| kws.iter().any(|kw| bio.contains(kw)))
        .map(|(p, _)| p.to_string())
}

fn extract_topics(bio: &str) -> Vec<String> {
    let bio = bio.to_lowercase();
    let table: [(&str, &[&str]); 10] = [
        ("technology", &["tech", "technology", "ai", "machine learning", "crypto", "blockchain"]),
        ("sports", &["sports", "football", "basketball", "soccer", "fitness", "gym"]),
        ("music", &["music", "musician", "singer", "band", "guitar", "piano"]),
        ("gaming", &["gaming", "gamer", "esports", "twitch", "streamer"]),
        ("politics", &["politics", "political", "democracy", "activist"]),
        ("business", &["business", "finance", "investing", "stocks", "trading"]),
        ("entertainment", &["movies", "films", "tv", "netflix", "entertainment"]),
        ("food", &["food", "cooking", "chef", "foodie", "restaurant"]),
        ("travel", &["travel", "traveler", "adventure", "exploring"]),
        ("art", &["art", "artist", "photography", "gallery"]),
    ];
    table
        .iter()
        .filter(|(_, kws)| kws.iter().any(|kw| bio.contains(kw)))
        .map(|(t, _)| t.to_string())
        .take(5)
        .collect()
}

fn sanitize_username(username: &str) -> String {
    let mut clean = String::new();
    for c in username.chars() {
        if c.is_alphanumeric() || c == '_' {
            clean.push(c);
        } else if !clean.ends_with('_') {
            clean.push('_');
        }
    }
    let clean: String = clean.trim_matches('_').chars().take(30).collect();
    if clean.is_empty() {
        "user".into()
    } else {
        clean
    }
}

fn sanitize_content(content: &str) -> String {
    content
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(2000)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crawl_fixture() -> CrawlResult {
        let user = ScrapedUser {
            platform: "telegram".into(),
            user_id: "durov".into(),
            username: "durov!".into(),
            display_name: "Pavel".into(),
            bio: "Founder and software engineer. Into crypto and travel.".into(),
            profile_url: None,
            avatar_url: None,
            followers: 1_200_000,
            following: 3,
            post_count: 42,
            verified: true,
        };
        let post = |author: &str, content: &str| ScrapedPost {
            platform: "telegram".into(),
            post_id: "1".into(),
            content: content.into(),
            author_id: author.into(),
            author_name: author.into(),
            timestamp: Utc::now(),
            url: None,
            likes: 5,
            shares: 1,
            comments: 2,
            views: 100,
            media_urls: vec![],
            hashtags: vec!["news".into()],
            mentions: vec![],
        };
        CrawlResult {
            platform: "telegram".into(),
            query: Some("durov".into()),
            posts: vec![post("durov", "hello   world"), post("someone_else", "hi")],
            users: vec![user],
            crawled_at: Utc::now(),
            success: true,
            error: None,
        }
    }

    #[test]
    fn seed_maps_users_and_posts() {
        let seed = create_seed(&crawl_fixture(), true, 100);

        // 1 scraped user + 1 extra author extracted from posts.
        assert_eq!(seed.profiles.len(), 2);
        assert_eq!(seed.original_post_count, 2);
        assert_eq!(seed.original_user_count, 1);

        let p0 = &seed.profiles[0];
        assert_eq!(p0.user_name, "user_00000"); // anonymized
        assert_eq!(p0.follower_count, 1_200_000);
        assert_eq!(p0.source_entity_uuid.as_deref(), Some("durov"));
        assert_eq!(p0.profession.as_deref(), Some("developer"));
        assert!(p0.interested_topics.contains(&"technology".to_string()));

        // Both posts map to a profile; whitespace collapsed; hashtags kept.
        assert_eq!(seed.initial_posts.len(), 2);
        assert_eq!(seed.initial_posts[0]["content"], "hello world");
        assert_eq!(seed.initial_posts[0]["user_id"], 0);
        assert_eq!(seed.initial_posts[0]["hashtags"][0], "news");
        assert_eq!(seed.initial_posts[1]["user_id"], 1);
    }

    #[test]
    fn non_anonymized_keeps_sanitized_names() {
        let seed = create_seed(&crawl_fixture(), false, 100);
        assert_eq!(seed.profiles[0].user_name, "durov");
        assert_eq!(seed.profiles[0].name, "Pavel");
    }

    #[test]
    fn max_profiles_is_respected() {
        let seed = create_seed(&crawl_fixture(), true, 1);
        assert_eq!(seed.profiles.len(), 1);
        // Post by unmapped author is dropped.
        assert_eq!(seed.initial_posts.len(), 1);
    }
}
