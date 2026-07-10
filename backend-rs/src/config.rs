use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub llm_api_key: String,
    pub llm_base_url: String,
    pub llm_model: String,
    // Optional "boost" model for heavier reasoning (report agent, persona synthesis).
    pub llm_boost_api_key: String,
    pub llm_boost_base_url: String,
    pub llm_boost_model: String,

    pub graph_db_path: String,
    /// Where the user-editable LLM provider settings persist (base_url/model/key
    /// per slot). Overlays the env defaults above at startup.
    pub settings_path: String,

    pub sim_data_dir: String,
    pub sim_default_max_rounds: u32,

    pub report_max_tool_calls: u32,
    pub report_max_reflection_rounds: u32,
    pub report_temperature: f32,

    pub tavily_api_key: String,

    pub port: u16,
}

fn var(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

impl Config {
    pub fn from_env() -> Self {
        // Load ../.env (project root) then backend-rs/.env, mirroring the Python precedence.
        let _ = dotenvy::from_filename("../.env");
        let _ = dotenvy::dotenv();

        let llm_api_key = var("LLM_API_KEY", "");
        let llm_base_url = var("LLM_BASE_URL", "https://api.openai.com/v1");
        let llm_model = var("LLM_MODEL_NAME", "gpt-4o-mini");

        Config {
            llm_boost_api_key: env::var("LLM_BOOST_API_KEY").unwrap_or_else(|_| llm_api_key.clone()),
            llm_boost_base_url: env::var("LLM_BOOST_BASE_URL").unwrap_or_else(|_| llm_base_url.clone()),
            llm_boost_model: env::var("LLM_BOOST_MODEL_NAME").unwrap_or_else(|_| llm_model.clone()),
            llm_api_key,
            llm_base_url,
            llm_model,

            graph_db_path: var("GRAPH_DB_PATH", "./uploads/graph.db"),
            settings_path: var("SETTINGS_PATH", "./uploads/settings.json"),

            sim_data_dir: var("OASIS_SIMULATION_DATA_DIR", "./uploads/simulations"),
            sim_default_max_rounds: var("OASIS_DEFAULT_MAX_ROUNDS", "10").parse().unwrap_or(10),

            report_max_tool_calls: var("REPORT_AGENT_MAX_TOOL_CALLS", "5").parse().unwrap_or(5),
            report_max_reflection_rounds: var("REPORT_AGENT_MAX_REFLECTION_ROUNDS", "2")
                .parse()
                .unwrap_or(2),
            report_temperature: var("REPORT_AGENT_TEMPERATURE", "0.5").parse().unwrap_or(0.5),

            tavily_api_key: var("TAVILY_API_KEY", ""),

            port: var("PORT", "5001").parse().unwrap_or(5001),
        }
    }
}
