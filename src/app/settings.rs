//! Shared settings application, reloads, and reversible theme preview.
use super::{App, EX_COMMANDS, ExSuggestion, Provider, ex_matches};

impl App {
    pub fn leader_label(&self) -> String {
        if self.leader == ' ' {
            "Space".into()
        } else {
            self.leader.to_string()
        }
    }
    pub fn ex_suggestions(&self, input: &str) -> Vec<ExSuggestion> {
        let input = input.trim_start();
        if input.starts_with("theme") && input.chars().nth(5).is_some_and(char::is_whitespace) {
            let prefix = input[5..].trim_start();
            return self
                .themes
                .iter()
                .filter(|name| name.starts_with(prefix))
                .map(|name| ExSuggestion {
                    completion: format!("theme {name}"),
                    label: name.clone(),
                    description: if *name == self.theme_name {
                        "Current theme".into()
                    } else if crate::config::is_builtin_theme(name) {
                        "Built-in theme".into()
                    } else {
                        "Custom theme".into()
                    },
                })
                .collect();
        }
        ex_matches(input)
            .into_iter()
            .map(|index| {
                let (name, description) = EX_COMMANDS[index];
                ExSuggestion {
                    completion: if input.chars().any(char::is_whitespace) {
                        input.trim().into()
                    } else {
                        name.into()
                    },
                    label: format!(":{name}"),
                    description: description.into(),
                }
            })
            .collect()
    }
    pub fn ex_menu_suggestions(&self, input: &str) -> Vec<ExSuggestion> {
        self.ex_suggestions(self.ex_completion_prefix.as_deref().unwrap_or(input))
    }
    /// Apply a validated configuration in one place for startup and reloads.
    pub fn apply_settings(
        &mut self,
        settings: crate::config::Config,
        theme: crate::config::Theme,
        provider: Option<Provider>,
        themes: Vec<String>,
    ) -> Result<(), String> {
        settings.validate().map_err(|error| error.to_string())?;
        let leader = settings
            .keys
            .character()
            .map_err(|error| error.to_string())?;
        self.ai.set_policy(settings.model.usage.clone())?;
        self.fallback_provider = provider.map(|provider| provider.with_usage(self.ai.clone()));
        self.provider = match (&self.auth, &self.chatgpt) {
            (Some(store), Some(connection)) if connection.plan => Some(
                Provider::chatgpt(
                    store.clone(),
                    connection.client_id.clone(),
                    connection.model.clone(),
                )
                .with_usage(self.ai.clone()),
            ),
            (_, Some(_)) => None,
            _ => self.fallback_provider.clone(),
        };
        self.theme_preview = None;
        self.theme = theme;
        self.theme_name = settings.theme;
        self.leader = leader;
        self.sidebar = settings.ui.sidebar;
        self.transparent_background = settings.ui.transparent_background;
        self.editor = settings.editor;
        self.themes = themes;
        Ok(())
    }
    pub fn reload_config(&mut self) {
        let result = (|| -> Result<(), String> {
            let (settings, theme) =
                crate::config::load(&self.config_path, true).map_err(|error| error.to_string())?;
            let provider = Provider::from_config(&settings.model)?;
            let themes =
                crate::config::theme_names(&self.config_path).map_err(|error| error.to_string())?;
            self.apply_settings(settings, theme, provider, themes)
        })();
        self.status = match result {
            Ok(()) => format!("Config reloaded · theme {}", self.theme_name),
            Err(error) => format!("Config reload failed: {error}. Previous settings kept."),
        };
    }
    pub(super) fn preview_theme(&mut self, name: &str) {
        match crate::config::load_theme(&self.config_path, name) {
            Ok(theme) => {
                if self.theme_preview.is_none() {
                    self.theme_preview = Some((self.theme_name.clone(), self.theme));
                }
                self.theme = theme;
                self.theme_name = crate::config::canonical_theme_name(name).into();
                self.status =
                    format!("Previewing {name} · Enter keeps it · Esc restores the previous theme");
            }
            Err(error) => self.status = format!("Theme preview failed: {error}"),
        }
    }
    pub(super) fn cancel_theme_preview(&mut self) {
        if let Some((name, theme)) = self.theme_preview.take() {
            self.theme_name = name;
            self.theme = theme;
            self.status = format!("Theme restored: {}", self.theme_name);
        }
    }
    pub fn apply_theme(&mut self, name: &str) {
        match crate::config::load_theme(&self.config_path, name) {
            Ok(theme) => {
                self.theme = theme;
                self.theme_name = crate::config::canonical_theme_name(name).into();
                self.status = format!("Theme: {name}");
            }
            Err(error) => self.status = format!("Theme failed: {error}. Previous theme kept."),
        }
    }
}
