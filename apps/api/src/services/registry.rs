//! THE generic in-memory registry (see CONTEXT.md "Registry"): a
//! process-lifetime `OnceLock<Mutex<HashMap<id, T>>>`. Graph-build tasks,
//! reports, and survey templates each hold their own typed static instance;
//! the locking/lifecycle logic lives only here.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Shared lifecycle vocabulary for registry entries. `Running` replaces the
/// old `Processing`/`Generating` synonyms; wire value is `"running"` (the
/// frontend only branches on `"completed"`/`"failed"`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

pub struct Registry<T>(OnceLock<Mutex<HashMap<String, T>>>);

impl<T> Default for Registry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Registry<T> {
    pub const fn new() -> Self {
        Registry(OnceLock::new())
    }
}

impl<T: Clone> Registry<T> {

    fn map(&self) -> MutexGuard<'_, HashMap<String, T>> {
        self.0.get_or_init(Default::default).lock().unwrap()
    }

    pub fn insert(&self, id: String, value: T) {
        self.map().insert(id, value);
    }

    pub fn get(&self, id: &str) -> Option<T> {
        self.map().get(id).cloned()
    }

    /// Apply `f` to the entry if present; returns whether it existed.
    pub fn update(&self, id: &str, f: impl FnOnce(&mut T)) -> bool {
        match self.map().get_mut(id) {
            Some(v) => {
                f(v);
                true
            }
            None => false,
        }
    }

    pub fn values(&self) -> Vec<T> {
        self.map().values().cloned().collect()
    }
}
