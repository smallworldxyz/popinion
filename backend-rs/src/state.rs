use crate::config::Config;
use crate::llm::Llm;
use std::sync::Arc;

/// Shared application state (cheap to clone; inner handles are Arc/pooled).
#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub llm: Llm,
    pub llm_boost: Llm,
    /// Neo4j graph handle. `None` when the DB was unreachable at startup;
    /// graph endpoints surface a clear error rather than panicking the server.
    pub neo4j: Option<neo4rs::Graph>,
    pub sims: crate::sim::SimRegistry,
}

impl AppState {
    pub async fn new(cfg: Config) -> Self {
        let llm = Llm::new(&cfg.llm_api_key, &cfg.llm_base_url, &cfg.llm_model);
        let llm_boost = Llm::new(&cfg.llm_boost_api_key, &cfg.llm_boost_base_url, &cfg.llm_boost_model);

        let neo4j = match neo4rs::Graph::new(&cfg.neo4j_uri, &cfg.neo4j_user, &cfg.neo4j_password).await {
            Ok(g) => {
                tracing::info!("connected to Neo4j at {}", cfg.neo4j_uri);
                Some(g)
            }
            Err(e) => {
                tracing::warn!("Neo4j unavailable ({e}); graph endpoints will error until it is up");
                None
            }
        };

        AppState {
            llm,
            llm_boost,
            neo4j,
            sims: crate::sim::SimRegistry::new(),
            cfg: Arc::new(cfg),
        }
    }

    pub fn graph(&self) -> crate::error::AppResult<&neo4rs::Graph> {
        self.neo4j
            .as_ref()
            .ok_or_else(|| crate::error::AppError::Other(anyhow::anyhow!("Neo4j not connected")))
    }

    pub fn sim_manager(&self) -> crate::sim::manager::Manager {
        crate::sim::manager::Manager::new(self.cfg.sim_data_dir.clone(), self.sims.clone(), self.llm.clone())
    }
}
