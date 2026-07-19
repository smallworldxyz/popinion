use crate::error::{AppError, AppResult, Success};
use crate::models::CrawlResult;
use crate::services::crawler::{
    bridge, facebook, google_trends, reddit, storage, telegram, threads, tiktok, twitter,
};
use crate::state::AppState;
use axum::extract::{Path, Query};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

/// Crawl endpoints:
///   POST /telegram  /twitter  /facebook  /reddit  /google-trends  /tiktok
///        /threads  /bridge
///   GET  /results  /results/:filename
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/telegram", post(crawl_telegram))
        .route("/twitter", post(crawl_twitter))
        .route("/facebook", post(crawl_facebook))
        .route("/reddit", post(crawl_reddit))
        .route("/google-trends", post(crawl_google_trends))
        .route("/tiktok", post(crawl_tiktok))
        .route("/threads", post(crawl_threads))
        .route("/results", get(list_results))
        .route("/results/:filename", get(get_result))
        .route("/bridge", post(bridge_to_simulation))
}

fn d50() -> usize {
    50
}
fn dtrue() -> bool {
    true
}

/// Shared payload for the three crawl endpoints: summary + optional
/// persistence info + sample posts. A failed crawl carries an `error` field.
fn crawl_response(result: &CrawlResult, save: bool, name_hint: &str) -> AppResult<Value> {
    let mut resp = json!({
        "platform": result.platform,
        "query": result.query,
        "posts_count": result.posts.len(),
        "users_count": result.users.len(),
        "crawled_at": result.crawled_at.to_rfc3339(),
    });
    if let Some(err) = &result.error {
        resp["error"] = json!(err);
    }
    if save && result.success {
        let (filename, path) = storage::save(&storage::default_dir(), result, name_hint)?;
        resp["saved_to"] = json!(path.display().to_string());
        resp["filename"] = json!(filename);
    }
    resp["sample_posts"] = serde_json::to_value(&result.posts[..result.posts.len().min(5)])
        .map_err(anyhow::Error::from)?;
    Ok(resp)
}

#[derive(Deserialize)]
struct TelegramReq {
    channel: Option<String>,
    #[serde(default = "d50")]
    max_posts: usize,
    #[serde(default = "dtrue")]
    save_result: bool,
}

async fn crawl_telegram(Json(req): Json<TelegramReq>) -> AppResult<Success<Value>> {
    let channel = req
        .channel
        .filter(|c| !c.trim().is_empty())
        .ok_or_else(|| AppError::BadRequest("channel is required".into()))?;
    let result = telegram::crawl_channel(&channel, req.max_posts).await;
    let mut resp = crawl_response(&result, req.save_result, &channel)?;
    resp["channel"] = json!(channel);
    Ok(Success(resp))
}

#[derive(Deserialize)]
struct TwitterReq {
    query: Option<String>,
    username: Option<String>,
    #[serde(default = "d50")]
    max_posts: usize,
    #[serde(default = "dtrue")]
    save_result: bool,
}

async fn crawl_twitter(Json(req): Json<TwitterReq>) -> AppResult<Success<Value>> {
    let (result, hint) = if let Some(username) = req.username.filter(|u| !u.trim().is_empty()) {
        (twitter::crawl_user(&username, req.max_posts).await, username)
    } else if let Some(query) = req.query.filter(|q| !q.trim().is_empty()) {
        (twitter::crawl_query(&query, req.max_posts).await, query)
    } else {
        return Err(AppError::BadRequest("query or username is required".into()));
    };
    Ok(Success(crawl_response(&result, req.save_result, &hint)?))
}

#[derive(Deserialize)]
struct FacebookReq {
    page: Option<String>,
    #[serde(default = "d50")]
    max_posts: usize,
    #[serde(default = "dtrue")]
    save_result: bool,
}

async fn crawl_facebook(Json(req): Json<FacebookReq>) -> AppResult<Success<Value>> {
    let page = req
        .page
        .filter(|p| !p.trim().is_empty())
        .ok_or_else(|| AppError::BadRequest("page is required".into()))?;
    let result = facebook::crawl_page(&page, req.max_posts).await;
    let mut resp = crawl_response(&result, req.save_result, &page)?;
    resp["page"] = json!(page);
    Ok(Success(resp))
}

#[derive(Deserialize)]
struct RedditReq {
    subreddit: Option<String>,
    query: Option<String>,
    #[serde(default = "d50")]
    max_posts: usize,
    #[serde(default = "dtrue")]
    save_result: bool,
}

async fn crawl_reddit(Json(req): Json<RedditReq>) -> AppResult<Success<Value>> {
    let (result, hint) = if let Some(subreddit) = req.subreddit.filter(|s| !s.trim().is_empty()) {
        (reddit::crawl_subreddit(&subreddit, req.max_posts).await, subreddit)
    } else if let Some(query) = req.query.filter(|q| !q.trim().is_empty()) {
        (reddit::crawl_query(&query, req.max_posts).await, query)
    } else {
        return Err(AppError::BadRequest("subreddit or query is required".into()));
    };
    Ok(Success(crawl_response(&result, req.save_result, &hint)?))
}

#[derive(Deserialize)]
struct GoogleTrendsReq {
    geo: Option<String>,
    #[serde(default = "d50")]
    max_posts: usize,
    #[serde(default = "dtrue")]
    save_result: bool,
}

async fn crawl_google_trends(Json(req): Json<GoogleTrendsReq>) -> AppResult<Success<Value>> {
    let geo = req
        .geo
        .filter(|g| !g.trim().is_empty())
        .unwrap_or_else(|| "US".into());
    let result = google_trends::crawl_trends(&geo, req.max_posts).await;
    let mut resp = crawl_response(&result, req.save_result, &geo)?;
    resp["geo"] = json!(geo);
    Ok(Success(resp))
}

#[derive(Deserialize)]
struct TiktokReq {
    username: Option<String>,
    query: Option<String>,
    #[serde(default = "d50")]
    max_posts: usize,
    #[serde(default = "dtrue")]
    save_result: bool,
}

async fn crawl_tiktok(Json(req): Json<TiktokReq>) -> AppResult<Success<Value>> {
    let (result, hint) = if let Some(username) = req.username.filter(|u| !u.trim().is_empty()) {
        (tiktok::crawl_user(&username, req.max_posts).await, username)
    } else if let Some(query) = req.query.filter(|q| !q.trim().is_empty()) {
        (tiktok::crawl_query(&query, req.max_posts).await, query)
    } else {
        return Err(AppError::BadRequest("username or query is required".into()));
    };
    Ok(Success(crawl_response(&result, req.save_result, &hint)?))
}

#[derive(Deserialize)]
struct ThreadsReq {
    username: Option<String>,
    #[serde(default = "d50")]
    max_posts: usize,
    #[serde(default = "dtrue")]
    save_result: bool,
}

async fn crawl_threads(Json(req): Json<ThreadsReq>) -> AppResult<Success<Value>> {
    let username = req
        .username
        .filter(|u| !u.trim().is_empty())
        .ok_or_else(|| AppError::BadRequest("username is required".into()))?;
    let result = threads::crawl_user(&username, req.max_posts).await;
    let mut resp = crawl_response(&result, req.save_result, &username)?;
    resp["username"] = json!(username);
    Ok(Success(resp))
}

#[derive(Deserialize)]
struct ListQuery {
    platform: Option<String>,
    #[serde(default = "d20")]
    limit: usize,
}
fn d20() -> usize {
    20
}

async fn list_results(Query(q): Query<ListQuery>) -> AppResult<Success<Value>> {
    let results = storage::list(&storage::default_dir(), q.platform.as_deref(), q.limit)?;
    Ok(Success(json!({
        "count": results.len(),
        "results": results,
    })))
}

async fn get_result(Path(filename): Path<String>) -> AppResult<Success<CrawlResult>> {
    if !storage::valid_filename(&filename) {
        return Err(AppError::BadRequest("invalid filename".into()));
    }
    let dir = storage::default_dir();
    if !storage::exists(&dir, &filename) {
        return Err(AppError::NotFound("File not found".into()));
    }
    Ok(Success(storage::load(&dir, &filename)?))
}

#[derive(Deserialize)]
struct BridgeReq {
    filename: Option<String>,
    /// Accepted for Python-API compatibility; only its basename is used, so
    /// reads stay confined to the crawl results directory.
    filepath: Option<String>,
    #[serde(default = "dtrue")]
    anonymize: bool,
    #[serde(default = "d100")]
    max_profiles: usize,
}
fn d100() -> usize {
    100
}

async fn bridge_to_simulation(Json(req): Json<BridgeReq>) -> AppResult<Success<Value>> {
    let filename = req
        .filename
        .or_else(|| {
            req.filepath
                .as_deref()
                .and_then(|p| p.rsplit(['/', '\\']).next())
                .map(str::to_string)
        })
        .ok_or_else(|| AppError::BadRequest("filepath or filename is required".into()))?;
    if !storage::valid_filename(&filename) {
        return Err(AppError::BadRequest("invalid filename".into()));
    }

    let dir = storage::default_dir();
    if !storage::exists(&dir, &filename) {
        return Err(AppError::NotFound("File not found".into()));
    }
    let result = storage::load(&dir, &filename)?;
    let seed = bridge::create_seed(&result, req.anonymize, req.max_profiles);

    Ok(Success(json!({
        "platform": seed.platform,
        "profiles_count": seed.profiles.len(),
        "initial_posts_count": seed.initial_posts.len(),
        "trending_topics": seed.trending_topics,
        "seed": seed,
    })))
}
