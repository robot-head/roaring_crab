use crate::hook_event::HookEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub master_volume: f32,
    pub muted: bool,
    pub enabled_hooks: HashMap<String, bool>,
}

impl Default for Config {
    fn default() -> Self {
        let mut enabled_hooks = HashMap::new();
        for h in HookEvent::ALL {
            enabled_hooks.insert(h.as_str().to_string(), true);
        }
        Self {
            master_volume: 0.7,
            muted: false,
            enabled_hooks,
        }
    }
}

impl Config {
    pub fn is_enabled(&self, hook: HookEvent) -> bool {
        self.enabled_hooks
            .get(hook.as_str())
            .copied()
            .unwrap_or(true)
    }

    pub fn load_or_default(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let cfg: Config = toml::from_str(&text).map_err(ConfigError::Parse)?;
                Ok(cfg)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let cfg = Config::default();
                cfg.save(path)?;
                Ok(cfg)
            }
            Err(e) => Err(ConfigError::Io(e)),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(ConfigError::Io)?;
        }
        let text = toml::to_string_pretty(self).map_err(ConfigError::Serialize)?;
        std::fs::write(path, text).map_err(ConfigError::Io)?;
        Ok(())
    }

    pub fn default_path() -> Option<std::path::PathBuf> {
        let dirs = directories::ProjectDirs::from("", "", "roaring-crab")?;
        Some(dirs.config_dir().join("config.toml"))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(std::io::Error),
    #[error("toml parse error: {0}")]
    Parse(toml::de::Error),
    #[error("toml serialize error: {0}")]
    Serialize(toml::ser::Error),
}
