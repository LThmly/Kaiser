//! Notebook lifecycle, source paths, and working directories.
use super::{Action, App, Focus, Modal, Notebook, PathBuf, Tab};

impl App {
    pub fn go_home(&mut self) {
        self.cancel_theme_preview();
        self.notebook_selected = false;
        self.revealed = false;
        self.editing = false;
        self.notes.normal();
        self.modal = None;
        self.modal_index = 0;
        self.ex_selected = None;
        self.ex_completion_prefix = None;
        self.focus = Focus::Workspace;
        self.scroll = None;
        self.scroll_max = 0;
        self.status = "Home".into();
    }
    pub(super) fn create_notebook(&mut self, name: String) {
        let mut notebook = Notebook::empty(name);
        if self.notebook_selected {
            notebook.directory = self.library.book().directory.clone();
        }
        self.library.notebooks.push(notebook);
        self.action(Action::Notebook(self.library.notebooks.len() - 1));
    }
    pub(super) fn source_path(&self, path: &std::path::Path) -> PathBuf {
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
        let text = path.to_string_lossy();
        let expanded = if text == "~" {
            home.map(PathBuf::from).unwrap_or_else(|| path.to_owned())
        } else if let Some(tail) = text.strip_prefix("~/").or_else(|| text.strip_prefix("~\\")) {
            home.map(PathBuf::from)
                .map(|home| home.join(tail))
                .unwrap_or_else(|| path.to_owned())
        } else {
            path.to_owned()
        };
        if expanded.is_absolute() {
            expanded
        } else {
            self.library.book().directory.join(expanded)
        }
    }
    pub fn import_matches(&self, query: &str) -> Result<Vec<crate::files::Entry>, String> {
        let query = query.trim();
        let path = self.source_path(std::path::Path::new(query));
        crate::files::matches(
            &path,
            query.is_empty()
                || query == "~"
                || query.ends_with(std::path::MAIN_SEPARATOR)
                || query.ends_with('/'),
        )
    }
    pub(super) fn change_directory(&mut self, path: &str) {
        if !self.require_notebook() {
            return;
        }
        let value = if path.is_empty() { "~" } else { path };
        let candidate = self.source_path(std::path::Path::new(value));
        match candidate.canonicalize() {
            Ok(directory) if directory.is_dir() => {
                self.status = format!("Directory: {}", directory.display());
                self.library.book_mut().directory = directory;
                self.dirty = true;
            }
            Ok(_) => self.status = "Directory unchanged: choose a directory, not a file".into(),
            Err(error) => self.status = format!("Directory unchanged: {error}"),
        }
    }
    pub fn new_notebook(&mut self) {
        self.editing = false;
        self.notes.normal();
        self.modal_index = 0;
        self.modal = Some(Modal::Notebook(String::new()));
    }
    pub fn pick_notebook(&mut self) {
        self.modal_index = self.library.active;
        self.modal = Some(Modal::SelectNotebook(String::new()));
    }
    pub(super) fn delete_notebook(&mut self, index: usize) {
        if index >= self.library.notebooks.len() {
            return;
        }
        let active = self.library.active;
        let removed = self.library.notebooks.remove(index);
        let ids: std::collections::HashSet<u64> =
            removed.cards.iter().map(|card| card.id).collect();
        for activity in &mut self.library.review_activity {
            activity.cards.retain(|id| !ids.contains(id));
        }
        self.library
            .review_activity
            .retain(|activity| !activity.cards.is_empty());
        if self
            .pending
            .as_ref()
            .is_some_and(|job| job.notebook == index)
        {
            self.cancel_generation();
        } else if let Some(job) = &mut self.pending
            && job.notebook > index
        {
            job.notebook -= 1;
        }
        if self
            .pending_import
            .as_ref()
            .is_some_and(|job| job.notebook == index)
        {
            self.pending_import = None;
        } else if let Some(job) = &mut self.pending_import
            && job.notebook > index
        {
            job.notebook -= 1;
        }
        self.library.active = if active > index {
            active - 1
        } else {
            active.min(self.library.notebooks.len().saturating_sub(1))
        };
        if active == index {
            self.go_home();
            self.notes = crate::notes::Editor::new();
            self.tab = Tab::Chat;
            self.input.clear();
            self.source = 0;
            self.card = 0;
            self.flashcard = 0;
        }
        self.dirty = true;
        self.status = format!("Deleted notebook: {}", removed.name);
    }
    pub fn notebook_matches(&self, query: &str) -> Vec<usize> {
        let query = query.trim().to_lowercase();
        self.library
            .notebooks
            .iter()
            .enumerate()
            .filter(|(_, book)| book.name.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect()
    }
    pub(super) fn require_notebook(&mut self) -> bool {
        if self.notebook_selected {
            true
        } else {
            self.pick_notebook();
            false
        }
    }
}
