use crate::hook_event::HookEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub master_volume: f32,
    pub muted: bool,
    /// Multiplier applied to the master volume for events that want the user's
    /// attention (Notification, Stop). Tool/info events use plain master volume.
    pub attention_volume_boost: f32,
    /// If `Some(n)`, the daemon re-plays the Notification patch every `n`
    /// seconds after a Notification fires, until any other hook event arrives
    /// (which clears the repeat). `None` disables repeating.
    pub notification_repeat_secs: Option<u32>,
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
            attention_volume_boost: 1.5,
            notification_repeat_secs: Some(30),
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

    /// Effective playback volume for a given hook event. Applies the
    /// attention boost to events that need user response. Output is clamped
    /// to [0, 1] so the daemon never gets fed >1.0 (which the mixer would
    /// clip anyway).
    pub fn volume_for(&self, hook: HookEvent) -> f32 {
        let base = self.master_volume;
        let scaled = if hook.is_attention() {
            base * self.attention_volume_boost
        } else {
            base
        };
        scaled.clamp(0.0, 1.0)
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
