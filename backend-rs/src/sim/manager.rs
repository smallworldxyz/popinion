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
    /// The knowledge graph this sim draws its population from (the "God's-eye"
    /// source). Set when created via the wizard; used by /prepare.
    #[serde(default)]
    pub graph_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
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

    /// Create a sim on disk. Personas may be empty when the sim is graph-linked
    /// and will be populated later via /prepare. Returns its id.
    pub fn create(
        &self,
        name: &str,
        profiles: Vec<AgentProfile>,
        config: SimConfig,
        graph_id: Option<String>,
        project_id: Option<String>,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let dir = self.dir(&id);
        std::fs::create_dir_all(&dir)?;
        let meta = SimMeta {
            simulation_id: id.clone(),
            name: name.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            num_agents: profiles.len(),
            status: "created".into(),
            graph_id,
            project_id,
        };
        self.write_profiles_and_config(&id, &profiles, config)?;
        self.write_meta(&meta)?;
        Ok(id)
    }

    /// Populate (or replace) a graph-linked sim's personas after /prepare.
    /// Preserves the sim's existing config (initial posts, timing) and only
    /// swaps the personas; bumps num_agents and marks it ready to start.
    pub fn attach_profiles(&self, id: &str, profiles: Vec<AgentProfile>) -> Result<()> {
        let dir = self.dir(id);
        if !dir.exists() {
            anyhow::bail!("simulation {id} not found");
        }
        // Keep the existing config; reset agent_configs so they regenerate for
        // the new persona set.
        let mut config: SimConfig = match std::fs::read(dir.join("config.json")) {
            Ok(raw) => serde_json::from_slice(&raw).unwrap_or_default(),
            Err(_) => SimConfig::default(),
        };
        config.agent_configs.clear();
        self.write_profiles_and_config(id, &profiles, config)?;
        let mut meta = self.meta(id)?;
        meta.num_agents = profiles.len();
        meta.status = "prepared".into();
        self.write_meta(&meta)?;
        Ok(())
    }

    fn write_profiles_and_config(&self, id: &str, profiles: &[AgentProfile], mut config: SimConfig) -> Result<()> {
        let dir = self.dir(id);
        config.simulation_id = id.to_string();
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
        std::fs::write(dir.join("profiles.json"), serde_json::to_vec_pretty(profiles)?)?;
        std::fs::write(dir.join("config.json"), serde_json::to_vec_pretty(&config)?)?;
        Ok(())
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
    /// `seed`/`permute` override the stored config for honesty-test runs.
    pub fn start(&self, id: &str, max_rounds: Option<u32>, seed: Option<u64>, permute: bool) -> Result<()> {
        if self.registry.get(id).is_some() {
            anyhow::bail!("simulation {id} already running");
        }
        let dir = self.dir(id);
        let profiles: Vec<AgentProfile> =
            serde_json::from_slice(&std::fs::read(dir.join("profiles.json"))?)?;
        let mut config: SimConfig = serde_json::from_slice(&std::fs::read(dir.join("config.json"))?)?;
        if seed.is_some() {
            config.seed = seed;
        }
        if permute {
            config.permute_personas = true;
        }
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
