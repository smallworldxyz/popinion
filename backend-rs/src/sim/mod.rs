pub mod action;
pub mod agent;
pub mod config;
pub mod engine;
pub mod manager;
pub mod persona;
pub mod store;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};

/// Commands sent into a running engine task (replaces the Python file-polling IPC).
pub enum Command {
    Interview {
        user_id: i64,
        prompt: String,
        reply: oneshot::Sender<anyhow::Result<String>>,
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
        self.status.lock().unwrap().clone()
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
        self.inner.lock().unwrap().get(id).cloned()
    }

    pub fn insert(&self, handle: SimHandle) {
        self.inner.lock().unwrap().insert(handle.simulation_id.clone(), handle);
    }

    pub fn remove(&self, id: &str) -> Option<SimHandle> {
        self.inner.lock().unwrap().remove(id)
    }

    pub fn ids(&self) -> Vec<String> {
        self.inner.lock().unwrap().keys().cloned().collect()
    }
}

impl Default for SimRegistry {
    fn default() -> Self {
        Self::new()
    }
}
