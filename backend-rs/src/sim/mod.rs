pub mod store;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Handle to a running simulation: cancellation + a channel to send interview
/// commands into the in-process engine task. Replaces the Python subprocess +
/// file-polling IPC entirely.
#[derive(Clone)]
pub struct SimHandle {
    pub simulation_id: String,
    pub status: Arc<Mutex<String>>,
    // Filled in when the engine is built (interview command sender, cancel token).
}

/// Registry of live simulations, keyed by simulation_id.
#[derive(Clone)]
pub struct SimRegistry {
    inner: Arc<Mutex<HashMap<String, SimHandle>>>,
}

impl SimRegistry {
    pub fn new() -> Self {
        SimRegistry {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get(&self, id: &str) -> Option<SimHandle> {
        self.inner.lock().unwrap().get(id).cloned()
    }

    pub fn insert(&self, handle: SimHandle) {
        self.inner
            .lock()
            .unwrap()
            .insert(handle.simulation_id.clone(), handle);
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
