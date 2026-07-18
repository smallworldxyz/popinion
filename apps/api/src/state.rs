use crate::config::Config;
use crate::llm::Llm;
use std::sync::{Arc, RwLock};

/// Shared application state (cheap to clone; inner handles are Arc/pooled).
#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    /// Runtime-swappable LLM provider settings (bulk + boost slots). The model
    /// selector writes here; `llm()`/`llm_boost()` build a client from the
    /// current values, so a provider change takes effect without a restart.
    pub llm_settings: Arc<RwLock<crate::settings::LlmSettings>>,
    /// ChatGPT-subscription OAuth credentials (the "Sign in with ChatGPT"
    /// provider). Shared so the login callback task can persist into it.
    pub chatgpt: Arc<crate::chatgpt_auth::ChatGptAuth>,
    /// Embedded graph store (SQLite) — no external database required.
    pub graph_db: crate::services::graph::db::GraphDb,
    pub sims: crate::sim::SimRegistry,
}

impl AppState {
    pub async fn new(cfg: Config) -> Self {
        let settings = crate::settings::LlmSettings::load(&cfg, std::path::Path::new(&cfg.settings_path));

        let graph_db = crate::services::graph::db::GraphDb::open(std::path::Path::new(&cfg.graph_db_path))
            .expect("open graph database");
        tracing::info!("graph store at {}", cfg.graph_db_path);

        let chatgpt = Arc::new(crate::chatgpt_auth::ChatGptAuth::load(&cfg.chatgpt_auth_path));

        AppState {
            llm_settings: Arc::new(RwLock::new(settings)),
            chatgpt,
            graph_db,
            sims: crate::sim::SimRegistry::new(),
            cfg: Arc::new(cfg),
        }
    }

    /// Build a client for a slot, routing to the ChatGPT subscription backend
    /// when its base_url is the sentinel, else the OpenAI-compatible API path.
    pub fn client_for(&self, slot: &crate::settings::LlmSlot) -> Llm {
        if crate::chatgpt_auth::is_chatgpt_backend(&slot.base_url) {
            Llm::chatgpt(&slot.model, self.chatgpt.clone())
        } else {
            Llm::new(&slot.api_key, &slot.base_url, &slot.model)
        }
    }

    /// A client for the bulk slot at the currently-configured provider.
    pub fn llm(&self) -> Llm {
        let s = self.llm_settings.read().unwrap();
        self.client_for(&s.bulk)
    }

    /// A client for the boost slot; falls back to the bulk slot when unset.
    pub fn llm_boost(&self) -> Llm {
        let s = self.llm_settings.read().unwrap();
        let slot = if s.boost.base_url.trim().is_empty() { &s.bulk } else { &s.boost };
        self.client_for(slot)
    }

    pub fn graph(&self) -> &crate::services::graph::db::GraphDb {
        &self.graph_db
    }

    pub fn sim_manager(&self) -> crate::sim::manager::Manager {
        crate::sim::manager::Manager::new(self.cfg.sim_data_dir.clone(), self.sims.clone(), self.llm())
    }
}
