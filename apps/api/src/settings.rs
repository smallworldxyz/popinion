//! User-editable LLM provider settings — the model selector's persistence.
//! Each slot (bulk / boost) is a plain OpenAI-compatible endpoint (base_url +
//! model + key), mirroring the two-slot design from `config.rs`. Env vars seed
//! the defaults; a saved `settings.json` overlays them so a user can switch
//! provider/model at runtime without touching `.env`.
//!
//! ponytail: keys live in this 0600 file for now (no worse than the plaintext
//! `.env` they replace). The OS keychain (Tauri desktop) is the Phase-2 upgrade
//! — swap the read/save of `api_key` for a keyring lookup then.

use crate::config::Config;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmSlot {
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub api_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmSettings {
    pub bulk: LlmSlot,
    pub boost: LlmSlot,
}

impl LlmSettings {
    /// Env defaults (from `Config`), overlaid by a saved `settings.json` if present.
    pub fn load(cfg: &Config, path: &Path) -> Self {
        let from_env = LlmSettings {
            bulk: LlmSlot {
                base_url: cfg.llm_base_url.clone(),
                model: cfg.llm_model.clone(),
                api_key: cfg.llm_api_key.clone(),
            },
            boost: LlmSlot {
                base_url: cfg.llm_boost_base_url.clone(),
                model: cfg.llm_boost_model.clone(),
                api_key: cfg.llm_boost_api_key.clone(),
            },
        };
        match std::fs::read(path) {
            Ok(raw) => serde_json::from_slice(&raw).unwrap_or(from_env),
            Err(_) => from_env,
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        write_owner_only(path, &serde_json::to_vec_pretty(self)?)
    }
}

/// Write a secrets file that is owner-only from the moment it exists. Creating
/// it first and chmod'ing after would leave the contents world-readable for the
/// window in between.
pub fn write_owner_only(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    f.write_all(bytes)?;
    // `mode` only applies on create; an existing file keeps its old permissions.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = f.set_permissions(std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> Config {
        // A minimal Config with just the fields load() reads.
        let mut c = Config::from_env();
        c.llm_base_url = "http://localhost:11434/v1".into();
        c.llm_model = "qwen2.5:7b".into();
        c.llm_api_key = "ollama".into();
        c.llm_boost_base_url = "https://api.openai.com/v1".into();
        c.llm_boost_model = "gpt-4o".into();
        c.llm_boost_api_key = "sk-secret".into();
        c
    }

    #[test]
    fn env_defaults_then_file_overlay_roundtrip() {
        let dir = std::env::temp_dir().join(format!("popinion-settings-{}", std::process::id()));
        let path = dir.join("settings.json");
        std::fs::remove_file(&path).ok();

        // No file → env defaults.
        let s = LlmSettings::load(&cfg(), &path);
        assert_eq!(s.bulk.model, "qwen2.5:7b");
        assert_eq!(s.boost.api_key, "sk-secret");

        // Save an override, reload → file wins.
        let mut edited = s;
        edited.bulk.model = "llama3.1:8b".into();
        edited.save(&path).unwrap();
        let reloaded = LlmSettings::load(&cfg(), &path);
        assert_eq!(reloaded.bulk.model, "llama3.1:8b");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "secrets file must be owner-only");
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
