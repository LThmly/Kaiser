//! User-facing configuration schema and defaults.
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub theme: String,
    pub keys: KeysConfig,
    pub ui: UiConfig,
    pub model: ModelConfig,
    pub editor: EditorConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "kaiser-tallgeese".into(),
            keys: KeysConfig::default(),
            ui: UiConfig::default(),
            model: ModelConfig::default(),
            editor: EditorConfig::default(),
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct EditorConfig {
    pub command: Vec<String>,
}
impl EditorConfig {
    pub fn argv(&self) -> io::Result<Vec<String>> {
        let command = if self.command.is_empty() {
            ["VISUAL", "EDITOR"]
                .into_iter()
                .filter_map(|key| std::env::var(key).ok())
                .find(|value| !value.trim().is_empty())
                .map(|value| value.split_whitespace().map(str::to_owned).collect())
                .unwrap_or_else(|| vec!["helix".into()])
        } else {
            self.command.clone()
        };
        if command.first().is_none_or(|name| name.trim().is_empty()) {
            return Err(io::Error::other(
                "editor.command must start with an executable name",
            ));
        }
        Ok(command)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct KeysConfig {
    pub leader: String,
}
impl Default for KeysConfig {
    fn default() -> Self {
        Self {
            leader: "space".into(),
        }
    }
}
impl KeysConfig {
    pub fn character(&self) -> io::Result<char> {
        if self.leader.eq_ignore_ascii_case("space") {
            return Ok(' ');
        }
        let mut chars = self.leader.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) if !c.is_control() => Ok(c),
            _ => Err(io::Error::other(
                "keys.leader must be 'space' or a single printable character, such as ','",
            )),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct UiConfig {
    pub sidebar: bool,
    pub transparent_background: bool,
}
impl Default for UiConfig {
    fn default() -> Self {
        Self {
            sidebar: true,
            transparent_background: false,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModelConfig {
    pub api_url: Option<String>,
    pub model: Option<String>,
    pub api_key_env: String,
    pub usage: crate::usage::Policy,
}
impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            api_url: None,
            model: None,
            api_key_env: "KAISER_API_KEY".into(),
            usage: crate::usage::Policy::default(),
        }
    }
}

impl Config {
    /// Validate settings before applying any part of a startup/reload operation.
    pub fn validate(&self) -> io::Result<()> {
        self.keys.character()?;
        self.editor.argv()?;
        self.model.usage.validate().map_err(io::Error::other)
    }
}
