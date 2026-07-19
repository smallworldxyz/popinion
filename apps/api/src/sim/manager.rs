use super::agent::Persona;
use super::config::SimConfig;
use super::engine::Engine;
use super::store::Store;
use super::SimRegistry;
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
    /// source). Set when created via the wizard; used by /prepare. This is the
    /// World a Run belongs to: Runs are grouped and listed by `graph_id`.
    #[serde(default)]
    pub graph_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    /// What kind of Run this is within its World. `canonical` is a first-class
    /// run; `alt`/`replicate`/`ablation` are derived from another via duplicate.
    /// Lets the runs table separate real runs from honesty-test spawns.
    #[serde(default = "default_kind")]
    pub kind: String,
    /// The Run this one was duplicated from, if any (fork/rehearsal lineage).
    #[serde(default)]
    pub parent_id: Option<String>,
}

fn default_kind() -> String {
    "canonical".into()
}

impl Manager {
    pub fn new(data_dir: impl Into<PathBuf>, registry: SimRegistry, llm: Llm) -> Self {
        Manager { data_dir: data_dir.into(), registry, llm }
    }

    fn dir(&self, id: &str) -> PathBuf {
        self.data_dir.join(id)
    }

    // The on-disk layout is known ONLY here; callers get paths or parsed
    // contents from these accessors, never by joining sim_data_dir themselves.
    pub fn db_path(&self, id: &str) -> PathBuf {
        self.dir(id).join("social.db")
    }
    pub fn profiles_path(&self, id: &str) -> PathBuf {
        self.dir(id).join("profiles.json")
    }
    pub fn config_path(&self, id: &str) -> PathBuf {
        self.dir(id).join("config.json")
    }

    pub fn load_personas(&self, id: &str) -> Result<Vec<Persona>> {
        let raw = std::fs::read(self.profiles_path(id))
            .with_context(|| format!("profiles for simulation {id} not found"))?;
        Ok(serde_json::from_slice(&raw)?)
    }

    pub fn load_config(&self, id: &str) -> Result<SimConfig> {
        let raw = std::fs::read(self.config_path(id))
            .with_context(|| format!("config for simulation {id} not found"))?;
        Ok(serde_json::from_slice(&raw)?)
    }

    /// Create a sim on disk. Personas may be empty when the sim is graph-linked
    /// and will be populated later via /prepare. Returns its id.
    pub fn create(
        &self,
        name: &str,
        profiles: Vec<Persona>,
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
            kind: default_kind(),
            parent_id: None,
        };
        self.write_profiles_and_config(&id, &profiles, config)?;
        self.write_meta(&meta)?;
        Ok(id)
    }

    /// Populate (or replace) a graph-linked sim's personas after /prepare.
    /// Preserves the sim's existing config (initial posts, timing) and only
    /// swaps the personas; bumps num_agents and marks it ready to start.
    /// `event`, when set, seeds the discussion with that post (the scenario the
    /// population reacts to) attributed to the first agent.
    pub fn attach_profiles(&self, id: &str, profiles: Vec<Persona>, event: Option<String>) -> Result<()> {
        if !self.dir(id).exists() {
            anyhow::bail!("simulation {id} not found");
        }
        // Keep the existing config; reset agent_configs so they regenerate for
        // the new persona set.
        let mut config = self.load_config(id).unwrap_or_default();
        config.agent_configs.clear();
        if let Some(content) = event.filter(|e| !e.trim().is_empty()) {
            config.event_config.initial_posts = vec![super::config::InitialPost {
                poster_agent_id: super::config::EVENT_AUTHOR_ID,
                content,
            }];
        }
        self.write_profiles_and_config(id, &profiles, config)?;
        let mut meta = self.meta(id)?;
        meta.num_agents = profiles.len();
        meta.status = "prepared".into();
        self.write_meta(&meta)?;
        Ok(())
    }

    /// A/B rehearsal: copy a prepared sim's population + config into a NEW sim
    /// whose only difference is the `event` under test. The source's effective
    /// seed is pinned into the copy so both runs draw the same randomness —
    /// the event stays the only variable.
    pub fn duplicate(&self, src_id: &str, event: &str) -> Result<String> {
        let profiles = self.load_personas(src_id)?;
        if profiles.is_empty() {
            anyhow::bail!("simulation {src_id} has no personas to duplicate; prepare it first");
        }
        let mut config = self.load_config(src_id)?;
        config.seed = Some(config.seed.unwrap_or_else(|| super::engine::derived_seed(src_id)));
        config.event_config.initial_posts = vec![super::config::InitialPost {
            poster_agent_id: super::config::EVENT_AUTHOR_ID,
            content: event.to_string(),
        }];
        let src = self.meta(src_id)?;
        let id = self.create(&format!("{} — alternative", src.name), profiles, config, src.graph_id, src.project_id)?;
        let mut meta = self.meta(&id)?;
        meta.status = "prepared".into();
        // A duplicate is a derived Run in the same World: record the lineage so
        // the runs table can separate it from canonical runs.
        meta.kind = "alt".into();
        meta.parent_id = Some(src_id.to_string());
        self.write_meta(&meta)?;
        Ok(id)
    }

    fn write_profiles_and_config(&self, id: &str, profiles: &[Persona], mut config: SimConfig) -> Result<()> {
        config.simulation_id = id.to_string();
        // Default one agent-config row per profile if none supplied.
        if config.agent_configs.is_empty() {
            config.agent_configs = profiles
                .iter()
                .map(|p| super::config::AgentConfig {
                    agent_id: p.user_id,
                    active_hours: super::config::default_active_hours(),
                    activity_level: super::config::DEFAULT_ACTIVITY_LEVEL,
                })
                .collect();
        }
        std::fs::write(self.profiles_path(id), serde_json::to_vec_pretty(profiles)?)?;
        std::fs::write(self.config_path(id), serde_json::to_vec_pretty(&config)?)?;
        Ok(())
    }

    fn write_meta(&self, meta: &SimMeta) -> Result<()> {
        std::fs::write(self.dir(&meta.simulation_id).join("metadata.json"), serde_json::to_vec_pretty(meta)?)?;
        Ok(())
    }

    /// Persist a terminal run status to disk. Reads the raw persisted meta (not
    /// the live-status-merged `meta()`), so the engine can stamp "completed" or
    /// "error: .." without clobbering it with the in-memory status.
    fn persist_status(&self, id: &str, status: &str) {
        let path = self.dir(id).join("metadata.json");
        if let Ok(raw) = std::fs::read(&path) {
            if let Ok(mut m) = serde_json::from_slice::<SimMeta>(&raw) {
                m.status = status.to_string();
                let _ = self.write_meta(&m);
            }
        }
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

    /// Open the social store for reads (or writes) of a sim. Errors if the sim
    /// doesn't exist, so read handlers 404 on a bogus id instead of silently
    /// materializing an empty social.db (and a directory) for it.
    pub fn store(&self, id: &str) -> Result<Arc<Store>> {
        if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
            anyhow::bail!("invalid simulation id");
        }
        if !self.dir(id).exists() {
            anyhow::bail!("simulation {id} not found");
        }
        Ok(Arc::new(Store::open(&self.db_path(id))?))
    }

    /// Spawn the engine for a sim. Idempotent-ish: errors if already running.
    /// `seed`/`permute` override the stored config for honesty-test runs.
    /// Returns the run's effective seed, which is also persisted to config.json
    /// so a later `duplicate` reproduces this run's randomness.
    pub fn start(&self, id: &str, max_rounds: Option<u32>, seed: Option<u64>, permute: bool) -> Result<u64> {
        if self.registry.get(id).is_some() {
            anyhow::bail!("simulation {id} already running");
        }
        let profiles = self.load_personas(id)?;
        let mut config = self.load_config(id)?;
        if seed.is_some() {
            config.seed = seed;
        }
        let effective_seed = config.seed.unwrap_or_else(|| super::engine::derived_seed(id));
        config.seed = Some(effective_seed);
        // Persist the seed (but not the ephemeral permute override) so the run
        // is reproducible after the fact.
        std::fs::write(self.config_path(id), serde_json::to_vec_pretty(&config)?)?;
        if permute {
            config.permute_personas = true;
        }
        let store = self.store(id)?;
        let engine = Engine::new(store, profiles, config, self.llm.clone());
        let mgr = self.clone();
        let sim_id = id.to_string();
        let persist: Arc<dyn Fn(&str) + Send + Sync> =
            Arc::new(move |status: &str| mgr.persist_status(&sim_id, status));
        let handle = engine.spawn(id.to_string(), max_rounds, persist);
        self.registry.insert(handle);
        Ok(effective_seed)
    }

    pub async fn stop(&self, id: &str) -> Result<()> {
        if let Some(h) = self.registry.remove(id) {
            h.stop().await;
        }
        Ok(())
    }

    /// Delete a simulation: stop it if live, then remove its directory to free
    /// disk and declutter the runs list. Irreversible. The id is validated the
    /// same way `store()` does so we can never remove a path outside data_dir.
    pub async fn delete(&self, id: &str) -> Result<()> {
        if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
            anyhow::bail!("invalid simulation id");
        }
        if let Some(h) = self.registry.remove(id) {
            h.stop().await;
        }
        let dir = self.dir(id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_manager() -> (Manager, PathBuf) {
        let dir = std::env::temp_dir().join(format!("popinion-mgr-test-{}", uuid::Uuid::new_v4()));
        let llm = Llm::new("", "http://localhost", "test");
        (Manager::new(&dir, SimRegistry::default(), llm), dir)
    }

    fn personas(n: usize) -> Vec<Persona> {
        serde_json::from_value(
            (0..n).map(|i| json!({ "user_id": i, "user_name": format!("u{i}") })).collect(),
        )
        .unwrap()
    }

    #[test]
    fn duplicate_copies_personas_and_swaps_only_the_event() {
        let (m, dir) = test_manager();
        let config = SimConfig {
            event_config: super::super::config::EventConfig {
                initial_posts: vec![super::super::config::InitialPost {
                    poster_agent_id: super::super::config::EVENT_AUTHOR_ID,
                    content: "policy A".into(),
                }],
            },
            ..Default::default()
        };
        let src = m.create("Land Reform", personas(3), config, None, None).unwrap();

        let dup = m.duplicate(&src, "policy B").unwrap();
        assert_ne!(dup, src, "duplicate must be an independent sim");

        // Same population — the point of the A/B rehearsal.
        let src_profiles = serde_json::to_value(m.load_personas(&src).unwrap()).unwrap();
        let dup_profiles = serde_json::to_value(m.load_personas(&dup).unwrap()).unwrap();
        assert_eq!(src_profiles, dup_profiles);

        // The event is the only variable: swapped in the copy, untouched at the source.
        let dup_cfg = m.load_config(&dup).unwrap();
        assert_eq!(dup_cfg.event_config.initial_posts[0].content, "policy B");
        let src_cfg = m.load_config(&src).unwrap();
        assert_eq!(src_cfg.event_config.initial_posts[0].content, "policy A");

        // The copy is pinned to the source's effective seed, so both runs draw
        // the same randomness despite different sim ids.
        assert_eq!(dup_cfg.seed, Some(super::super::engine::derived_seed(&src)));

        // Ready to start straight away, with the full roster.
        let meta = m.meta(&dup).unwrap();
        assert_eq!(meta.status, "prepared");
        assert_eq!(meta.num_agents, 3);

        // Lineage: the copy is a derived run pointing back at its source, while
        // the source stays canonical. The runs table groups both under one World.
        assert_eq!(meta.kind, "alt");
        assert_eq!(meta.parent_id.as_deref(), Some(src.as_str()));
        assert_eq!(m.meta(&src).unwrap().kind, "canonical");
        assert_eq!(m.meta(&src).unwrap().parent_id, None);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn persist_status_survives_a_dropped_handle() {
        // A finished run must read as terminal on disk, not revert to the stale
        // "prepared" once the live handle is gone (the runs-table bug).
        let (m, dir) = test_manager();
        let id = m.create("Fuel Levy", personas(2), SimConfig::default(), None, None).unwrap();
        assert_eq!(m.meta(&id).unwrap().status, "created");

        m.persist_status(&id, "completed");
        assert_eq!(m.meta(&id).unwrap().status, "completed");

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn duplicate_refuses_an_unprepared_sim() {
        let (m, dir) = test_manager();
        let src = m.create("empty", vec![], SimConfig::default(), Some("g1".into()), None).unwrap();
        assert!(m.duplicate(&src, "policy B").is_err(), "no personas to copy");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn duplicate_keeps_an_explicitly_seeded_source_seed() {
        let (m, dir) = test_manager();
        let config = SimConfig { seed: Some(42), ..Default::default() };
        let src = m.create("seeded", personas(2), config, None, None).unwrap();
        let dup = m.duplicate(&src, "policy B").unwrap();
        assert_eq!(m.load_config(&dup).unwrap().seed, Some(42));
        let _ = std::fs::remove_dir_all(dir);
    }
}
