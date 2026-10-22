//! Workspace navigation, note editing, and flashcard interaction.
use super::{Action, App, Card, Focus, KeyCode, KeyEvent, KeyModifiers, Modal, Tab};

impl App {
    pub fn cards(&self) -> Vec<&Card> {
        self.library
            .book()
            .cards
            .iter()
            .filter(|c| !self.drafts_only || c.draft)
            .collect()
    }
    pub fn current_card(&self) -> Option<&Card> {
        self.cards().get(self.card).copied()
    }
    pub fn select_card(&mut self, index: usize) {
        self.card = index.min(self.cards().len().saturating_sub(1));
        if let Some(id) = self.current_card().map(|c| c.id) {
            self.flashcard = self
                .library
                .book()
                .cards
                .iter()
                .position(|c| c.id == id)
                .unwrap_or(0);
        }
        self.revealed = false;
        self.scroll = None;
    }
    pub fn view_panel_card(&mut self, index: usize, focus: Focus) {
        self.drafts_only = false;
        self.card = index;
        self.open(Tab::Cards);
        self.focus = focus;
    }
    pub fn open(&mut self, tab: Tab) {
        if !self.require_notebook() {
            return;
        }
        self.tab = tab;
        self.focus = Focus::Workspace;
        self.editing = false;
        self.notes.normal();
        self.scroll = if tab == Tab::Notes { Some(0) } else { None };
        if tab == Tab::Cards {
            self.select_card(self.card);
        }
    }
    pub fn read_source(&mut self) {
        if self.require_notebook() {
            self.editing = false;
            self.notes.normal();
            self.modal_index = 0;
            self.modal = Some(Modal::Reader);
        }
    }
    pub(super) fn toggle_notes_preview(&mut self) {
        if !self.require_notebook() {
            return;
        }
        if self.tab != Tab::Notes {
            self.open(Tab::Notes);
        }
        self.focus = Focus::Workspace;
        self.notes.preview = !self.notes.preview;
        self.notes.normal();
        self.editing = false;
        self.scroll = Some(0);
    }
    pub(super) fn notes_key(&mut self, key: KeyEvent) {
        self.notes.preview = false;
        let changed = self.notes.key(&mut self.library.book_mut().notes, key);
        self.editing = self.notes.mode == crate::notes::Mode::Insert;
        self.dirty |= changed;
    }
    pub fn move_selection(&mut self, down: bool) {
        let step = |value: usize, len: usize| {
            if len == 0 {
                0
            } else if down {
                (value + 1).min(len - 1)
            } else {
                value.saturating_sub(1)
            }
        };
        match self.focus {
            Focus::Notebooks => {}
            Focus::Sources => self.source = step(self.source, self.library.book().sources.len()),
            Focus::Workspace if self.tab == Tab::Cards => {
                self.select_card(step(self.card, self.cards().len()));
            }
            Focus::Flashcards => {
                let index = step(self.flashcard, self.library.book().cards.len());
                self.view_panel_card(index, Focus::Flashcards);
            }
            _ => self.scroll_by(if down { 3 } else { -3 }),
        }
    }
    pub fn scroll_by(&mut self, amount: i32) {
        if self.tab == Tab::Notes && !self.notes.preview && self.focus == Focus::Workspace {
            let code = if amount < 0 {
                KeyCode::Up
            } else {
                KeyCode::Down
            };
            for _ in 0..amount.unsigned_abs() {
                self.notes.key(
                    &mut self.library.book_mut().notes,
                    KeyEvent::new(code, KeyModifiers::NONE),
                );
            }
            return;
        }
        if self.focus == Focus::Flashcards {
            let index = self
                .flashcard
                .saturating_add_signed(amount as isize)
                .min(self.library.book().cards.len().saturating_sub(1));
            self.view_panel_card(index, Focus::Flashcards);
            return;
        }
        let current = self.scroll.unwrap_or(self.scroll_max);
        self.scroll = Some(
            current
                .saturating_add_signed(amount as isize)
                .min(self.scroll_max),
        );
    }
    pub fn draft(&mut self) {
        if !self.require_notebook() {
            return;
        }
        let book = self.library.book();
        let assistant = book.messages.iter().rev().find(|m| m.role == "assistant");
        let front = book
            .messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.text.clone())
            .unwrap_or_default();
        let citation = assistant.and_then(|m| m.citations.first()).cloned();
        let back = citation
            .as_ref()
            .map(|c| c.passage.clone())
            .or_else(|| assistant.map(|m| m.text.clone()))
            .unwrap_or_default();
        self.modal = Some(Modal::Card {
            id: None,
            front,
            back,
            field: 0,
            citation,
        });
    }
    pub fn edit_panel_card(&mut self) {
        if self.library.book().cards.is_empty() {
            self.draft();
            return;
        }
        self.drafts_only = false;
        self.card = self.flashcard.min(self.library.book().cards.len() - 1);
        self.open(Tab::Cards);
        self.edit_card();
    }
    pub fn edit_card(&mut self) {
        if let Some(card) = self.current_card().cloned() {
            self.modal = Some(Modal::Card {
                id: Some(card.id),
                front: card.front,
                back: card.back,
                field: 0,
                citation: card.citation,
            });
        } else {
            self.draft();
        }
    }
    pub fn search(&self, query: &str) -> Vec<(String, Action)> {
        if query.trim().is_empty() {
            return vec![];
        }
        let query = query.to_lowercase();
        let mut matches = vec![];
        for (i, s) in self.library.book().sources.iter().enumerate() {
            if s.name.to_lowercase().contains(&query) || s.content.to_lowercase().contains(&query) {
                matches.push((format!("SOURCE  {}", s.name), Action::ReadSource(i)));
            }
        }
        for (i, c) in self.library.book().cards.iter().enumerate() {
            if c.front.to_lowercase().contains(&query) || c.back.to_lowercase().contains(&query) {
                matches.push((format!("CARD    {}", c.front), Action::OpenCard(i)));
            }
        }
        matches
    }
}
