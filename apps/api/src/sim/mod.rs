pub mod action;
pub mod agent;
pub mod config;
pub mod engine;
pub mod manager;
pub mod classify;
pub mod persona;
pub mod stance;
pub mod store;
pub mod validate;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};

/// Commands sent into a running engine task.
pub enum Command {
    Interview {
        user_id: i64,
        prompt: String,
        reply: oneshot::Sender<anyhow::Result<String>>,
    },
    /// Drop a post into the live discussion mid-run (disinformation red-team,
    /// a policy reversal). Replies with (post_id, round it landed at).
    InjectPost {
        content: String,
        reply: oneshot::Sender<anyhow::Result<(i64, i64)>>,
    },
    Stop,
}

/// Handle to a running simulation: live status + a channel into the engine task.
#[derive(Clone)]
pub struct SimHandle {
    pub simulation_id: String,
    pub status: Arc<Mutex<String>>,
    pub cmd: mpsc::Sender<Command>,
}

impl SimHandle {
    pub fn status(&self) -> String {
        self.status.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

/// Registry of live simulations, keyed by simulation_id.
#[derive(Clone)]
pub struct SimRegistry {
    inner: Arc<Mutex<HashMap<String, SimHandle>>>,
}

impl SimRegistry {
    pub fn new() -> Self {
        SimRegistry { inner: Arc::new(Mutex::new(HashMap::new())) }
    }

    pub fn get(&self, id: &str) -> Option<SimHandle> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).get(id).cloned()
    }

    pub fn insert(&self, handle: SimHandle) {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).insert(handle.simulation_id.clone(), handle);
    }

    pub fn remove(&self, id: &str) -> Option<SimHandle> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).remove(id)
    }
}

impl Default for SimRegistry {
    fn default() -> Self {
        Self::new()
    }
}
