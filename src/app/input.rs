//! Terminal key routing and paste handling for navigation and editing.
use super::{App, Focus, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, Modal, Tab};

impl App {
    pub fn key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if matches!(key.code, KeyCode::Char('f' | 'n' | 'h' | '?'))
            && key.modifiers.contains(KeyModifiers::SUPER)
        {
            if key.kind != KeyEventKind::Repeat {
                if key.code == KeyCode::Char('?') && matches!(self.modal, Some(Modal::Card { .. }))
                {
                    self.status =
                        "Finish or cancel card editing before opening another menu.".into();
                } else {
                    self.cancel_theme_preview();
                    self.editing = false;
                    self.notes.normal();
                    self.ex_selected = None;
                    self.ex_completion_prefix = None;
                    self.modal_index = 0;
                    match key.code {
                        KeyCode::Char('f') => self.pick_notebook(),
                        KeyCode::Char('n') => self.new_notebook(),
                        KeyCode::Char('h') => self.go_home(),
                        KeyCode::Char('?') => self.modal = Some(Modal::Help),
                        _ => unreachable!(),
                    }
                }
            }
            return;
        }
        if self.modal.is_some() {
            self.modal_key(key);
            return;
        }
        if self.notebook_selected && self.tab == Tab::Notes && self.editing {
            self.notes.mode = crate::notes::Mode::Insert;
            self.notes_key(key);
            return;
        }
        if self.editing {
            match key.code {
                KeyCode::Esc => {
                    self.editing = false;
                    self.notes.normal();
                    self.status = "Navigation mode".into();
                }
                KeyCode::Enter
                    if self.tab == Tab::Chat && key.modifiers.contains(KeyModifiers::SHIFT) =>
                {
                    self.input.push('\n')
                }
                KeyCode::Enter if self.tab == Tab::Chat => self.send(),
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    self.input.push(c);
                }
                _ => {}
            }
            return;
        }
        if key.code == KeyCode::Char(':')
            && !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            if let Ok(themes) = crate::config::theme_names(&self.config_path) {
                self.themes = themes;
            }
            self.ex_history_index = self.ex_history.len();
            self.ex_selected = None;
            self.ex_completion_prefix = None;
            self.modal = Some(Modal::Ex(String::new()));
            return;
        }
        if key.code == KeyCode::Char(self.leader)
            && !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            && key.kind != KeyEventKind::Repeat
        {
            self.modal_index = 0;
            self.modal = Some(Modal::Leader);
            return;
        }
        if !self.notebook_selected {
            match key.code {
                KeyCode::Enter | KeyCode::Char('i') | KeyCode::F(2..=6) => self.pick_notebook(),
                KeyCode::Tab | KeyCode::BackTab => {
                    self.focus = if self.focus == Focus::Notebooks {
                        Focus::Workspace
                    } else {
                        Focus::Notebooks
                    }
                }
                KeyCode::F(1) | KeyCode::Char('?') => {
                    self.modal_index = 0;
                    self.modal = Some(Modal::Help);
                }
                KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.modal_index = 0;
                    self.modal = Some(Modal::Palette(String::new()));
                }
                _ => {}
            }
            return;
        }
        if self.tab == Tab::Notes
            && self.focus == Focus::Workspace
            && (!self.notes.preview
                || matches!(key.code, KeyCode::Char('i' | 'a' | 'I' | 'A' | 'o' | 'O')))
            && !matches!(key.code, KeyCode::Tab | KeyCode::BackTab | KeyCode::F(_))
            && !(key.code == KeyCode::Char('k') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            self.notes_key(key);
            return;
        }
        match key.code {
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.modal_index = 0;
                self.modal = Some(Modal::Palette(String::new()));
            }
            KeyCode::Tab => self.focus = self.focus.next(false),
            KeyCode::BackTab => self.focus = self.focus.next(true),
            KeyCode::F(1) | KeyCode::Char('?') => {
                self.modal_index = 0;
                self.modal = Some(Modal::Help);
            }
            KeyCode::F(2) => self.open(Tab::Chat),
            KeyCode::F(3) => {
                self.drafts_only = false;
                self.open(Tab::Cards);
            }
            KeyCode::F(4) => self.open(Tab::Cards),
            KeyCode::F(5) => self.open(Tab::Notes),
            KeyCode::F(6) => self.read_source(),
            KeyCode::F(10) => self.maximized = !self.maximized,
            KeyCode::Char('e') if self.focus == Focus::Flashcards => self.edit_panel_card(),
            KeyCode::Char('e') if self.tab == Tab::Cards => self.edit_card(),
            KeyCode::Char('i') => {
                if !matches!(self.tab, Tab::Chat | Tab::Notes) {
                    self.open(Tab::Chat);
                }
                self.focus = Focus::Workspace;
                self.editing = true;
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(false),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(true),
            KeyCode::PageUp => self.scroll_by(-10),
            KeyCode::PageDown => self.scroll_by(10),
            KeyCode::Left
                if self.tab == Tab::Cards
                    && matches!(self.focus, Focus::Workspace | Focus::Flashcards) =>
            {
                self.select_card(self.card.saturating_sub(1))
            }
            KeyCode::Right
                if self.tab == Tab::Cards
                    && matches!(self.focus, Focus::Workspace | Focus::Flashcards) =>
            {
                self.select_card(self.card + 1)
            }
            KeyCode::Home if self.focus == Focus::Flashcards => {
                self.view_panel_card(0, Focus::Flashcards)
            }
            KeyCode::Home if self.tab == Tab::Cards => self.select_card(0),
            KeyCode::End if self.tab == Tab::Cards && self.focus == Focus::Workspace => {
                self.select_card(self.cards().len().saturating_sub(1))
            }
            KeyCode::End if self.focus == Focus::Flashcards => {
                self.view_panel_card(
                    self.library.book().cards.len().saturating_sub(1),
                    Focus::Flashcards,
                );
            }
            KeyCode::Home => {
                self.scroll = Some(0);
            }
            KeyCode::End => self.scroll = None,
            KeyCode::Enter | KeyCode::Char('x') if self.focus == Focus::Sources => {
                if key.code == KeyCode::Enter {
                    self.read_source();
                } else if let Some(source) = self.library.book_mut().sources.get_mut(self.source) {
                    source.selected = !source.selected;
                    self.dirty = true;
                }
            }
            KeyCode::Enter if self.focus == Focus::Flashcards => {
                if self.library.book().cards.is_empty() {
                    self.draft();
                } else {
                    self.view_panel_card(self.flashcard, Focus::Workspace);
                }
            }
            KeyCode::Enter if self.focus == Focus::Notebooks => self.pick_notebook(),
            KeyCode::Enter if self.tab == Tab::Cards => {
                self.revealed = !self.revealed;
                if self.revealed
                    && let Some(id) = self
                        .current_card()
                        .filter(|card| !card.draft)
                        .map(|card| card.id)
                    && crate::progress::record(&mut self.library, crate::progress::today(), id)
                {
                    self.dirty = true;
                }
            }
            KeyCode::Enter if self.tab == Tab::Chat => {
                self.focus = Focus::Workspace;
                self.editing = true;
            }
            _ => {}
        }
    }
    pub fn paste(&mut self, text: &str) {
        if matches!(self.modal, Some(Modal::Import(_))) {
            self.modal_index = 0;
        }
        if matches!(self.modal, Some(Modal::Ex(_))) {
            self.cancel_theme_preview();
            self.ex_selected = None;
            self.ex_completion_prefix = None;
        }
        // Strip terminal control characters; preserve newlines only in multiline fields.
        let clean: String = text
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
            .collect();
        match &mut self.modal {
            Some(
                Modal::Import(s)
                | Modal::Notebook(s)
                | Modal::Search(s)
                | Modal::Palette(s)
                | Modal::SelectNotebook(s)
                | Modal::Ex(s),
            ) => s.push_str(&clean.replace(['\n', '\t'], " ")),
            Some(Modal::Card {
                front, back, field, ..
            }) => {
                if *field == 0 {
                    front.push_str(&clean);
                } else {
                    back.push_str(&clean);
                }
            }
            _ if self.editing && self.tab == Tab::Notes => {
                self.notes
                    .insert(&mut self.library.book_mut().notes, &clean);
                self.dirty = true;
            }
            _ if self.editing && self.tab == Tab::Chat => {
                self.input.push_str(&clean.replace('\t', "    "))
            }
            _ => {}
        }
    }
}
