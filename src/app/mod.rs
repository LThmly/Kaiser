//! Application state and action dispatch. Feature behavior lives in child modules.
mod accounts;
mod commands;
mod generation;
mod input;
mod modals;
mod notebooks;
mod settings;
mod sources;
mod state;
#[cfg(test)]
mod tests;
mod workspace;

pub use commands::{COMMANDS, Command, EX_COMMANDS, ExSuggestion, LEADER_BINDINGS, ex_matches};
pub use state::*;

use crate::{
    model::{Card, Library, Message, Notebook},
    provider::{Provider, local_answer},
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use std::{path::PathBuf, sync::mpsc};

pub struct App {
    pub logo: Option<ratatui_image::protocol::StatefulProtocol>,
    pub theme: crate::config::Theme,
    pub theme_name: String,
    theme_preview: Option<(String, crate::config::Theme)>,
    pub config_path: PathBuf,
    pub editor: crate::config::EditorConfig,
    pub themes: Vec<String>,
    pub open_config_requested: Option<EditorTarget>,
    pub leader: char,
    pub library: Library,
    pub provider: Option<Provider>,
    pub demo_mode: bool,
    pub ai: crate::usage::Runtime,
    fallback_provider: Option<Provider>,
    pub auth: Option<crate::auth::Store>,
    pub accounts: Vec<crate::auth::Summary>,
    pub chatgpt: Option<crate::auth::Connection>,
    pub chatgpt_models: Vec<crate::auth::Model>,
    pub login: Option<crate::auth::Login>,
    account_task: Option<mpsc::Receiver<Result<AccountTaskResult, String>>>,
    pub tab: Tab,
    pub focus: Focus,
    pub source: usize,
    pub notebook_selected: bool,
    pub card: usize,
    pub drafts_only: bool,
    pub input: String,
    pub editing: bool,
    pub notes: crate::notes::Editor,
    pub modal: Option<Modal>,
    pub modal_index: usize,
    pub flashcard: usize,
    pub revealed: bool,
    pub scroll: Option<usize>,
    pub scroll_max: usize,
    pub maximized: bool,
    pub sidebar: bool,
    pub transparent_background: bool,
    pub dirty: bool,
    pub save_requested: bool,
    pub ex_history: Vec<String>,
    pub ex_history_index: usize,
    pub ex_selected: Option<usize>,
    pub ex_completion_prefix: Option<String>,
    pub quit: bool,
    pub status: String,
    pub pending: Option<Pending>,
    pub generation_progress: String,
    progress_receiver: Option<mpsc::Receiver<String>>,
    pub pending_import: Option<PendingImport>,
    pub hits: Vec<(Rect, Action)>,
}

impl App {
    pub fn new(library: Library, provider: Option<Provider>) -> Self {
        let defaults = crate::config::Config::default();
        let ai = provider
            .as_ref()
            .map(|provider| provider.usage.clone())
            .unwrap_or_default();
        Self {
            logo: None,
            theme: crate::config::Theme::default(),
            theme_name: defaults.theme,
            theme_preview: None,
            config_path: crate::config::default_path(),
            editor: defaults.editor,
            themes: crate::config::builtin_theme_names(),
            open_config_requested: None,
            leader: defaults
                .keys
                .character()
                .expect("default leader must be valid"),
            library,
            fallback_provider: provider.clone(),
            provider,
            demo_mode: false,
            ai,
            auth: None,
            accounts: vec![],
            chatgpt: None,
            chatgpt_models: vec![],
            login: None,
            account_task: None,
            tab: Tab::Chat,
            focus: Focus::Workspace,
            source: 0,
            notebook_selected: false,
            card: 0,
            drafts_only: false,
            input: String::new(),
            editing: false,
            notes: crate::notes::Editor::new(),
            modal: None,
            modal_index: 0,
            flashcard: 0,
            revealed: false,
            scroll: None,
            scroll_max: 0,
            maximized: false,
            sidebar: defaults.ui.sidebar,
            transparent_background: defaults.ui.transparent_background,
            dirty: false,
            save_requested: false,
            ex_history: vec![],
            ex_history_index: 0,
            ex_selected: None,
            ex_completion_prefix: None,
            quit: false,
            status: String::new(),
            pending: None,
            generation_progress: String::new(),
            progress_receiver: None,
            pending_import: None,
            hits: vec![],
        }
    }
    pub fn action(&mut self, action: Action) {
        match action {
            Action::ConfirmSummary(confirm) => {
                if let Some(Modal::SummaryPlan {
                    cards,
                    fingerprint,
                    requests,
                    ..
                }) = self.modal.take()
                    && confirm
                {
                    self.run_approved_plan(cards, fingerprint, requests);
                }
            }
            Action::ComposeChat => {
                if self.require_notebook() {
                    self.tab = Tab::Chat;
                    self.focus = Focus::Workspace;
                    self.editing = true;
                }
            }
            Action::SendChat => self.send(),
            Action::Focus(focus) => {
                self.focus = focus;
                if focus != Focus::Workspace {
                    self.editing = false;
                }
            }
            Action::Notebook(i) => {
                self.notebook_selected = true;
                self.modal = None;
                self.library.active = i;
                self.notes = crate::notes::Editor::new();
                self.revealed = false;
                self.source = 0;
                self.card = 0;
                self.flashcard = 0;
                self.open(Tab::Chat);
                self.status = format!("Opened {}", self.library.book().name);
                self.dirty = true;
            }
            Action::Source(i) => {
                self.editing = false;
                self.source = i;
                self.focus = Focus::Sources;
            }
            Action::PickNotebook => self.pick_notebook(),
            Action::ConfirmNotebookDeletion(confirm) => {
                if matches!(self.modal, Some(Modal::DeleteNotebook { .. })) {
                    self.modal_key(KeyEvent::new(
                        KeyCode::Char(if confirm { 'y' } else { 'n' }),
                        KeyModifiers::NONE,
                    ));
                }
            }
            Action::Command(index) => self.command(index),
            Action::Tab(tab) => self.open(tab),
            Action::ReadSource(i) => {
                self.source = i;
                self.read_source();
            }
            Action::OpenCard(i) => {
                self.drafts_only = false;
                self.card = i;
                self.open(Tab::Cards);
            }
            Action::SelectFlashcard(i) => self.view_panel_card(i, Focus::Flashcards),
        }
    }
}
