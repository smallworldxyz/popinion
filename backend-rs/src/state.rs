use crate::config::Config;
use crate::llm::Llm;
use std::sync::Arc;

/// Shared application state (cheap to clone; inner handles are Arc/pooled).
#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub llm: Llm,
    pub llm_boost: Llm,
    /// Embedded graph store (SQLite) — no external database required.
    pub graph_db: crate::services::graph::db::GraphDb,
    pub sims: crate::sim::SimRegistry,
}

impl AppState {
    pub async fn new(cfg: Config) -> Self {
        let llm = Llm::new(&cfg.llm_api_key, &cfg.llm_base_url, &cfg.llm_model);
        let llm_boost = Llm::new(&cfg.llm_boost_api_key, &cfg.llm_boost_base_url, &cfg.llm_boost_model);

        let graph_db = crate::services::graph::db::GraphDb::open(std::path::Path::new(&cfg.graph_db_path))
            .expect("open graph database");
        tracing::info!("graph store at {}", cfg.graph_db_path);

        AppState {
            llm,
            llm_boost,
            graph_db,
            sims: crate::sim::SimRegistry::new(),
            cfg: Arc::new(cfg),
        }
    }

    pub fn graph(&self) -> &crate::services::graph::db::GraphDb {
        &self.graph_db
    }

    pub fn sim_manager(&self) -> crate::sim::manager::Manager {
        crate::sim::manager::Manager::new(self.cfg.sim_data_dir.clone(), self.sims.clone(), self.llm.clone())
    }
}
