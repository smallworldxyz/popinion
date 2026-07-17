//! "Sign in with ChatGPT" — use a ChatGPT subscription as an LLM provider.
//!
//! This is the same OAuth flow the Codex CLI uses: PKCE against
//! `auth.openai.com`, tokens persisted locally, and inference against the
//! private ChatGPT backend (`chatgpt.com/backend-api/codex/responses`) — NOT the
//! metered `/v1` API. It exists so a flat monthly ChatGPT plan can power
//! Popinion instead of pay-per-token API credits.
//!
//! Reality check (surface these to users, not just the code):
//! - It reuses Codex's public OAuth client id and an UNDOCUMENTED private
//!   endpoint. OpenAI can change or lock it at any time, and using it outside
//!   the Codex CLI is against OpenAI's terms. Personal use on your own account.
//! - Access tokens expire in hours; we refresh with the stored refresh token.
//!
//! ponytail: tokens live in a 0600 JSON file, same trust level as the existing
//! settings.json that holds API keys. Keychain is the desktop Phase-2 upgrade.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Mutex;

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const ISSUER: &str = "https://auth.openai.com";
const REDIRECT_PORT: u16 = 1455;
const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const SCOPE: &str = "openid profile email offline_access api.connectors.read api.connectors.invoke";
const ORIGINATOR: &str = "codex_cli_rs";

/// The ChatGPT backend base URL — the sentinel a slot's `base_url` is set to
/// when it should authenticate via subscription instead of an API key.
pub const BACKEND: &str = "https://chatgpt.com/backend-api/codex";

/// True when this base_url means "use the ChatGPT subscription backend".
pub fn is_chatgpt_backend(base_url: &str) -> bool {
    base_url.trim_end_matches('/') == BACKEND
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct Creds {
    access_token: String,
    refresh_token: String,
    account_id: String,
    #[serde(default)]
    email: String,
    #[serde(default)]
    plan: String,
}

#[derive(Clone, Default)]
pub enum LoginStatus {
    #[default]
    Idle,
    Pending,
    Done,
    Failed(String),
}

/// Owns the persisted ChatGPT credentials plus the in-flight login state.
pub struct ChatGptAuth {
    path: PathBuf,
    creds: Mutex<Option<Creds>>,
    status: Mutex<LoginStatus>,
    /// The waiting-for-callback task. It owns :1455 until the redirect arrives,
    /// so an abandoned login would hold the port forever — keep the handle to
    /// cancel it and let the user retry.
    pending: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl ChatGptAuth {
    pub fn load(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let creds = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Creds>(&b).ok());
        ChatGptAuth {
            path,
            creds: Mutex::new(creds),
            status: Mutex::new(LoginStatus::Idle),
            pending: Mutex::new(None),
        }
    }

    /// Cancel a login that's still sitting on :1455, freeing the port. Returns
    /// true if one was actually waiting.
    fn cancel_pending(&self) -> bool {
        match self.pending.lock().unwrap().take() {
            Some(h) => {
                h.abort();
                true
            }
            None => false,
        }
    }

    pub fn logged_in(&self) -> bool {
        self.creds.lock().unwrap().as_ref().is_some_and(|c| !c.refresh_token.is_empty())
    }

    /// { logged_in, email, plan, status } for the settings UI.
    pub fn status_json(&self) -> Value {
        let c = self.creds.lock().unwrap();
        let (email, plan) = c.as_ref().map(|c| (c.email.clone(), c.plan.clone())).unwrap_or_default();
        let status = match &*self.status.lock().unwrap() {
            LoginStatus::Idle => "idle".to_string(),
            LoginStatus::Pending => "pending".to_string(),
            LoginStatus::Done => "done".to_string(),
            LoginStatus::Failed(e) => format!("failed: {e}"),
        };
        serde_json::json!({
            "logged_in": c.as_ref().is_some_and(|c| !c.refresh_token.is_empty()),
            "email": email,
            "plan": plan,
            "status": status,
        })
    }

    pub fn logout(&self) {
        self.cancel_pending(); // don't leave :1455 held by a login we're abandoning
        *self.creds.lock().unwrap() = None;
        *self.status.lock().unwrap() = LoginStatus::Idle;
        std::fs::remove_file(&self.path).ok();
    }

    fn persist(&self, creds: Creds) {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(&creds) {
            if std::fs::write(&self.path, bytes).is_ok() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600));
                }
            }
        }
        *self.creds.lock().unwrap() = Some(creds);
    }

    /// A valid (access_token, account_id), refreshing if the token is near
    /// expiry. Called on every backend request.
    pub async fn valid(&self) -> Result<(String, String)> {
        let creds = self
            .creds
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| anyhow!("not signed in to ChatGPT — sign in from Settings"))?;

        // Refresh when the access token expires within 60s (or can't be read).
        let fresh = match jwt_exp(&creds.access_token) {
            Some(exp) if exp > now() + 60 => creds,
            _ => {
                let refreshed = refresh(&creds.refresh_token).await.context("refresh ChatGPT token")?;
                let merged = Creds {
                    access_token: refreshed.access_token,
                    // The auth server rotates the refresh token; keep the old one
                    // if the response omits it.
                    refresh_token: if refreshed.refresh_token.is_empty() {
                        creds.refresh_token
                    } else {
                        refreshed.refresh_token
                    },
                    account_id: refreshed.account_id.unwrap_or(creds.account_id),
                    email: refreshed.email.unwrap_or(creds.email),
                    plan: refreshed.plan.unwrap_or(creds.plan),
                };
                self.persist(merged.clone());
                merged
            }
        };
        Ok((fresh.access_token, fresh.account_id))
    }

    /// Start the browser OAuth flow: bind the callback listener, spawn the task
    /// that waits for the redirect and exchanges the code, and return the URL
    /// the frontend should open. `self` must be an `Arc` for the spawned task.
    pub async fn begin_login(self: std::sync::Arc<Self>) -> Result<String> {
        let pkce = Pkce::new();
        let state = uuid::Uuid::new_v4().to_string();

        // Retrying a login is the normal case (user closed the tab, took too
        // long, picked the wrong account) — drop our own abandoned attempt
        // rather than reporting our own listener as "port busy".
        let had_pending = self.cancel_pending();

        // The redirect_uri registered for this client id is fixed to :1455, so
        // this port is not negotiable. abort() frees it asynchronously; give the
        // runtime a moment before deciding someone else really owns it.
        let listener = bind_callback(had_pending).await?;

        let url = authorize_url(&pkce.challenge, &state);
        *self.status.lock().unwrap() = LoginStatus::Pending;

        let this = self.clone();
        let verifier = pkce.verifier.clone();
        let expected_state = state.clone();
        let handle = tokio::spawn(async move {
            match await_callback(listener, &expected_state).await {
                Ok(code) => match exchange_code(&code, &verifier).await {
                    Ok(creds) => {
                        this.persist(creds);
                        *this.status.lock().unwrap() = LoginStatus::Done;
                    }
                    Err(e) => *this.status.lock().unwrap() = LoginStatus::Failed(e.to_string()),
                },
                Err(e) => *this.status.lock().unwrap() = LoginStatus::Failed(e.to_string()),
            }
            this.pending.lock().unwrap().take();
        });
        *self.pending.lock().unwrap() = Some(handle);

        Ok(url)
    }
}

/// Bind the fixed callback port. When we just aborted our own listener the port
/// frees asynchronously, so retry briefly before blaming another process.
async fn bind_callback(retry: bool) -> Result<tokio::net::TcpListener> {
    let attempts = if retry { 20 } else { 1 };
    let mut last = None;
    for _ in 0..attempts {
        match tokio::net::TcpListener::bind(("127.0.0.1", REDIRECT_PORT)).await {
            Ok(l) => return Ok(l),
            Err(e) => {
                last = Some(e);
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
        }
    }
    Err(anyhow!(
        "cannot start login server on :{REDIRECT_PORT} (is Codex or another login running?): {}",
        last.expect("at least one attempt")
    ))
}

// ---- OAuth PKCE ----

struct Pkce {
    verifier: String,
    challenge: String,
}

impl Pkce {
    fn new() -> Self {
        // 64 bytes of entropy from four v4 UUIDs — avoids an rng dependency.
        let mut raw = Vec::with_capacity(64);
        for _ in 0..4 {
            raw.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
        }
        let verifier = b64url(&raw);
        let challenge = b64url(&Sha256::digest(verifier.as_bytes()));
        Pkce { verifier, challenge }
    }
}

fn authorize_url(challenge: &str, state: &str) -> String {
    let params = [
        ("response_type", "code"),
        ("client_id", CLIENT_ID),
        ("redirect_uri", REDIRECT_URI),
        ("scope", SCOPE),
        ("code_challenge", challenge),
        ("code_challenge_method", "S256"),
        ("id_token_add_organizations", "true"),
        ("codex_cli_simplified_flow", "true"),
        ("state", state),
        ("originator", ORIGINATOR),
    ];
    // reqwest re-exports url::Url, whose parse_with_params percent-encodes values.
    reqwest::Url::parse_with_params(&format!("{ISSUER}/oauth/authorize"), params)
        .expect("valid authorize url")
        .to_string()
}

/// Accept connections on :1455 until the OAuth redirect arrives; return `code`.
async fn await_callback(listener: tokio::net::TcpListener, expected_state: &str) -> Result<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    loop {
        let (mut sock, _) = listener.accept().await?;
        let mut buf = [0u8; 4096];
        let n = sock.read(&mut buf).await.unwrap_or(0);
        let req = String::from_utf8_lossy(&buf[..n]);
        // First line: "GET /auth/callback?code=...&state=... HTTP/1.1"
        let Some(path) = req.lines().next().and_then(|l| l.split_whitespace().nth(1)) else {
            continue;
        };
        if !path.starts_with("/auth/callback") {
            let _ = sock.write_all(http_page("Not found")).await;
            continue;
        }
        let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");
        let mut code = None;
        let mut state_ok = false;
        for (k, v) in reqwest::Url::parse(&format!("http://x/?{query}"))
            .map(|u| u.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect::<Vec<_>>())
            .unwrap_or_default()
        {
            match k.as_str() {
                "code" => code = Some(v),
                "state" => state_ok = v == expected_state,
                _ => {}
            }
        }
        let _ = sock
            .write_all(http_page("Signed in to ChatGPT. You can close this tab and return to Popinion."))
            .await;
        let _ = sock.flush().await;
        if !state_ok {
            return Err(anyhow!("login state mismatch (possible CSRF) — try again"));
        }
        return code.ok_or_else(|| anyhow!("no authorization code in callback"));
    }
}

fn http_page(msg: &str) -> &'static [u8] {
    // Static body; the message is the same shape regardless of branch, so leak a
    // fixed page. ponytail: not worth threading a dynamic body through a raw socket.
    let _ = msg;
    b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n<html><body style=\"font-family:sans-serif;padding:3rem;text-align:center\"><h2>Signed in to ChatGPT</h2><p>You can close this tab and return to Popinion.</p></body></html>"
}

/// Parsed token response, with account/email/plan pulled from the id_token JWT.
struct ExchangedCreds {
    access_token: String,
    refresh_token: String,
    account_id: Option<String>,
    email: Option<String>,
    plan: Option<String>,
}

async fn exchange_code(code: &str, verifier: &str) -> Result<Creds> {
    let resp = reqwest::Client::new()
        .post(format!("{ISSUER}/oauth/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", REDIRECT_URI),
            ("client_id", CLIENT_ID),
            ("code_verifier", verifier),
        ])
        .send()
        .await?;
    let ex = parse_token_response(resp).await?;
    Ok(Creds {
        access_token: ex.access_token,
        refresh_token: ex.refresh_token,
        account_id: ex.account_id.unwrap_or_default(),
        email: ex.email.unwrap_or_default(),
        plan: ex.plan.unwrap_or_default(),
    })
}

async fn refresh(refresh_token: &str) -> Result<ExchangedCreds> {
    let resp = reqwest::Client::new()
        .post(format!("{ISSUER}/oauth/token"))
        .json(&serde_json::json!({
            "client_id": CLIENT_ID,
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
        }))
        .send()
        .await?;
    parse_token_response(resp).await
}

async fn parse_token_response(resp: reqwest::Response) -> Result<ExchangedCreds> {
    let status = resp.status();
    let v: Value = if status.is_success() {
        resp.json().await.context("decode token response")?
    } else {
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("token endpoint {status}: {}", body.chars().take(200).collect::<String>()));
    };
    let access_token = v["access_token"].as_str().unwrap_or_default().to_string();
    let refresh_token = v["refresh_token"].as_str().unwrap_or_default().to_string();
    let id_token = v["id_token"].as_str().unwrap_or_default();
    let (account_id, email, plan) = id_token_claims(id_token);
    if access_token.is_empty() {
        return Err(anyhow!("token response missing access_token"));
    }
    Ok(ExchangedCreds { access_token, refresh_token, account_id, email, plan })
}

// ---- JWT helpers (no verification — we only read claims from our own tokens) ----

/// Pull chatgpt_account_id / email / plan from the id_token's OpenAI auth claims.
fn id_token_claims(jwt: &str) -> (Option<String>, Option<String>, Option<String>) {
    let Some(claims) = jwt_payload(jwt) else {
        return (None, None, None);
    };
    let auth = &claims["https://api.openai.com/auth"];
    let account_id = auth["chatgpt_account_id"].as_str().map(String::from);
    let plan = auth["chatgpt_plan_type"].as_str().map(String::from);
    let email = claims["email"]
        .as_str()
        .or_else(|| claims["https://api.openai.com/profile"]["email"].as_str())
        .map(String::from);
    (account_id, email, plan)
}

fn jwt_exp(jwt: &str) -> Option<i64> {
    jwt_payload(jwt)?["exp"].as_i64()
}

fn jwt_payload(jwt: &str) -> Option<Value> {
    let payload_b64 = jwt.split('.').nth(1)?;
    let bytes = b64url_decode(payload_b64)?;
    serde_json::from_slice(&bytes).ok()
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ---- base64url (no padding) — a few lines beats a dependency ----

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn b64url(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        out.push(B64[(n >> 18 & 63) as usize] as char);
        out.push(B64[(n >> 12 & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64[(n >> 6 & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(B64[(n & 63) as usize] as char);
        }
    }
    out
}

fn b64url_decode(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| B64.iter().position(|&b| b == c).map(|p| p as u32);
    let chars: Vec<u8> = s.bytes().filter(|&c| c != b'=').collect();
    let mut out = Vec::with_capacity(chars.len() * 3 / 4);
    for chunk in chars.chunks(4) {
        let mut n = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            n |= val(c)? << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64url_roundtrips() {
        for data in [&b""[..], b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar", &[0u8, 255, 128, 1, 2]] {
            let enc = b64url(data);
            assert!(!enc.contains('=') && !enc.contains('+') && !enc.contains('/'));
            assert_eq!(b64url_decode(&enc).unwrap(), data, "roundtrip {enc}");
        }
    }

    #[test]
    fn reads_claims_from_a_jwt() {
        // Hand-build header.payload.sig with url-safe payload; sig is ignored.
        let payload = serde_json::json!({
            "exp": 1234567890,
            "email": "a@b.com",
            "https://api.openai.com/auth": { "chatgpt_account_id": "acct_123", "chatgpt_plan_type": "plus" }
        });
        let jwt = format!("h.{}.s", b64url(payload.to_string().as_bytes()));
        assert_eq!(jwt_exp(&jwt), Some(1234567890));
        let (acct, email, plan) = id_token_claims(&jwt);
        assert_eq!(acct.as_deref(), Some("acct_123"));
        assert_eq!(email.as_deref(), Some("a@b.com"));
        assert_eq!(plan.as_deref(), Some("plus"));
    }

    #[test]
    fn detects_backend_sentinel() {
        assert!(is_chatgpt_backend(BACKEND));
        assert!(is_chatgpt_backend(&format!("{BACKEND}/")));
        assert!(!is_chatgpt_backend("https://api.openai.com/v1"));
    }
}
