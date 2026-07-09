use super::agent::AgentProfile;
use super::config::SimConfig;
use super::engine::Engine;
use super::store::Store;
use super::{SimHandle, SimRegistry};
use crate::llm::Llm;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// Filesystem + registry lifecycle for simulations. Each sim owns a directory
/// `{data_dir}/{sim_id}/` holding profiles.json, config.json, metadata.json and
/// social.db. Read endpoints open the DB directly (WAL) — no shared writer.
#[derive(Clone)]
pub struct Manager {
    data_dir: PathBuf,
    registry: SimRegistry,
    llm: Llm,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimMeta {
    pub simulation_id: String,
    pub name: String,
    pub created_at: String,
    pub num_agents: usize,
    #[serde(default)]
    pub status: String,
}

impl Manager {
    pub fn new(data_dir: impl Into<PathBuf>, registry: SimRegistry, llm: Llm) -> Self {
        Manager { data_dir: data_dir.into(), registry, llm }
    }

    fn dir(&self, id: &str) -> PathBuf {
        self.data_dir.join(id)
    }
    fn db_path(&self, id: &str) -> PathBuf {
        self.dir(id).join("social.db")
    }

    /// Create a sim on disk from personas + optional config. Returns its id.
    pub fn create(&self, name: &str, profiles: Vec<AgentProfile>, mut config: SimConfig) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let dir = self.dir(&id);
        std::fs::create_dir_all(&dir)?;
        config.simulation_id = id.clone();
        // Default one agent-config row per profile if none supplied.
        if config.agent_configs.is_empty() {
            config.agent_configs = profiles
                .iter()
                .map(|p| super::config::AgentConfig {
                    agent_id: p.user_id,
                    active_hours: (8..23).collect(),
                    activity_level: 0.5,
                })
                .collect();
        }
        std::fs::write(dir.join("profiles.json"), serde_json::to_vec_pretty(&profiles)?)?;
        std::fs::write(dir.join("config.json"), serde_json::to_vec_pretty(&config)?)?;
        let meta = SimMeta {
            simulation_id: id.clone(),
            name: name.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            num_agents: profiles.len(),
            status: "created".into(),
        };
        self.write_meta(&meta)?;
        Ok(id)
    }

    fn write_meta(&self, meta: &SimMeta) -> Result<()> {
        std::fs::write(self.dir(&meta.simulation_id).join("metadata.json"), serde_json::to_vec_pretty(meta)?)?;
        Ok(())
    }

    pub fn meta(&self, id: &str) -> Result<SimMeta> {
        let raw = std::fs::read(self.dir(id).join("metadata.json"))
            .with_context(|| format!("simulation {id} not found"))?;
        let mut m: SimMeta = serde_json::from_slice(&raw)?;
        // Live status wins over the persisted one.
        if let Some(h) = self.registry.get(id) {
            m.status = h.status();
        }
        Ok(m)
    }

    pub fn list(&self) -> Vec<SimMeta> {
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&self.data_dir) {
            for e in rd.flatten() {
                if let Some(name) = e.file_name().to_str() {
                    if let Ok(m) = self.meta(name) {
                        out.push(m);
                    }
                }
            }
        }
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        out
    }

    /// Open the social store for reads (or writes) of a sim.
    pub fn store(&self, id: &str) -> Result<Arc<Store>> {
        Ok(Arc::new(Store::open(&self.db_path(id))?))
    }

    /// Spawn the engine for a sim. Idempotent-ish: errors if already running.
    pub fn start(&self, id: &str, max_rounds: Option<u32>) -> Result<()> {
        if self.registry.get(id).is_some() {
            anyhow::bail!("simulation {id} already running");
        }
        let dir = self.dir(id);
        let profiles: Vec<AgentProfile> =
            serde_json::from_slice(&std::fs::read(dir.join("profiles.json"))?)?;
        let config: SimConfig = serde_json::from_slice(&std::fs::read(dir.join("config.json"))?)?;
        let store = self.store(id)?;
        let engine = Engine::new(store, profiles, config, self.llm.clone());
        let handle = engine.spawn(id.to_string(), max_rounds);
        self.registry.insert(handle);
        Ok(())
    }

    pub async fn stop(&self, id: &str) -> Result<()> {
        if let Some(h) = self.registry.remove(id) {
            h.stop().await;
        }
        Ok(())
    }

    pub fn handle(&self, id: &str) -> Option<SimHandle> {
        self.registry.get(id)
    }
}
