//! Per-modal input handlers and explicit modal dispatch.
use super::{
    Action, App, COMMANDS, Card, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, Modal, PathBuf,
};

impl App {
    pub fn modal_key(&mut self, key: KeyEvent) {
        let Some(modal) = self.modal.take() else {
            return;
        };
        // Deletion has its own Esc behavior: return to the filtered picker.
        if matches!(modal, Modal::DeleteNotebook { .. }) {
            self.delete_notebook_key(key, modal);
            return;
        }
        if key.code == KeyCode::Esc {
            if matches!(modal, Modal::SignIn(_)) {
                self.login = None;
                self.status = "ChatGPT sign-in cancelled".into();
            }
            if matches!(modal, Modal::ChatgptWelcome)
                && let Some(store) = &self.auth
                && let Err(error) = store.welcome_seen()
            {
                self.status = error;
            }
            if matches!(modal, Modal::Ex(_)) {
                self.cancel_theme_preview();
                self.ex_selected = None;
                self.ex_completion_prefix = None;
            }
            return;
        }
        match modal {
            Modal::SummaryPlan { .. } => self.summary_plan_key(key, modal),
            Modal::AiUsage(_) => self.usage_key(key, modal),
            Modal::Accounts | Modal::ChatgptModels | Modal::ChatgptWelcome | Modal::SignIn(_) => {
                self.account_key(key, modal)
            }
            Modal::Import(_) => self.import_key(key, modal),
            Modal::Ex(_) => self.ex_key(key, modal),
            Modal::SelectNotebook(_) => self.notebook_picker_key(key, modal),
            Modal::Reader => self.reader_key(key, modal),
            Modal::Help => self.help_key(key, modal),
            Modal::Leader => self.leader_key(key, modal),
            Modal::Card { .. } => self.card_key(key, modal),
            Modal::Notebook(_) | Modal::Search(_) | Modal::Palette(_) => self.text_key(key, modal),
            Modal::DeleteNotebook { .. } => unreachable!("deletion handled before cancellation"),
        }
    }
    fn delete_notebook_key(&mut self, key: KeyEvent, modal: Modal) {
        if let Modal::DeleteNotebook {
            notebook,
            query,
            selected,
        } = &modal
        {
            let cancel = matches!(key.code, KeyCode::Esc | KeyCode::Char('n'))
                || (key.code == KeyCode::Enter && self.modal_index == 0);
            let confirm = key.code == KeyCode::Char('y')
                || (key.code == KeyCode::Enter && self.modal_index == 1);
            if cancel || confirm {
                let (notebook, mut query, selected) = (*notebook, query.clone(), *selected);
                if confirm {
                    self.delete_notebook(notebook);
                }
                if self.library.notebooks.is_empty() {
                    self.modal = None;
                    return;
                }
                if confirm && self.notebook_matches(&query).is_empty() {
                    query.clear();
                }
                self.modal_index =
                    selected.min(self.notebook_matches(&query).len().saturating_sub(1));
                self.modal = Some(Modal::SelectNotebook(query));
            } else {
                if matches!(
                    key.code,
                    KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right
                ) {
                    self.modal_index = 1 - self.modal_index.min(1);
                }
                self.modal = Some(modal);
            }
        }
    }
    fn summary_plan_key(&mut self, key: KeyEvent, modal: Modal) {
        if let Modal::SummaryPlan {
            cards,
            fingerprint,
            requests,
            ..
        } = &modal
        {
            match key.code {
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right => {
                    self.modal_index = 1 - self.modal_index.min(1)
                }
                KeyCode::Enter => {
                    if self.modal_index == 1 {
                        self.run_approved_plan(cards.clone(), fingerprint.clone(), *requests);
                    }
                    return;
                }
                _ => {}
            }
            self.modal = Some(modal);
        }
    }
    fn usage_key(&mut self, key: KeyEvent, modal: Modal) {
        if matches!(modal, Modal::AiUsage(_)) {
            match key.code {
                KeyCode::Enter => return,
                KeyCode::Down | KeyCode::Char('j') => {
                    self.modal_index = self.modal_index.saturating_add(1)
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.modal_index = self.modal_index.saturating_sub(1)
                }
                KeyCode::PageDown => self.modal_index = self.modal_index.saturating_add(10),
                KeyCode::PageUp => self.modal_index = self.modal_index.saturating_sub(10),
                KeyCode::Home => self.modal_index = 0,
                KeyCode::End => self.modal_index = usize::MAX,
                _ => {}
            }
            self.modal = Some(modal);
        }
    }
    fn account_key(&mut self, key: KeyEvent, modal: Modal) {
        if matches!(
            modal,
            Modal::Accounts | Modal::ChatgptModels | Modal::ChatgptWelcome | Modal::SignIn(_)
        ) {
            let count = if matches!(modal, Modal::Accounts) {
                self.accounts.len() + 1
            } else {
                self.chatgpt_models.len()
            };
            match key.code {
                KeyCode::Tab | KeyCode::Down if count > 0 => {
                    self.modal_index = (self.modal_index + 1) % count
                }
                KeyCode::BackTab | KeyCode::Up if count > 0 => {
                    self.modal_index = (self.modal_index + count - 1) % count
                }
                KeyCode::Enter => match modal {
                    Modal::Accounts => {
                        if let Some(account) = self.accounts.get(self.modal_index).cloned() {
                            if account.connected {
                                match self.auth.as_ref().unwrap().select(&account.client_id) {
                                    Ok(connection) => self.connect_account(connection),
                                    Err(error) => self.status = error,
                                }
                            } else {
                                self.start_login(Some(account.client_id));
                            }
                        } else {
                            self.start_login(None);
                        }
                        return;
                    }
                    Modal::ChatgptModels => {
                        if let Some(model) = self.chatgpt_models.get(self.modal_index) {
                            match self.auth.as_ref().unwrap().set_model(&model.slug) {
                                Ok(()) => {
                                    if let Some(connection) = &mut self.chatgpt {
                                        connection.model = Some(model.slug.clone());
                                    }
                                    if let Some(provider) = &mut self.provider {
                                        provider.model = model.slug.clone();
                                    }
                                    self.status = format!("ChatGPT model: {}", model.name);
                                }
                                Err(error) => self.status = error,
                            }
                            return;
                        }
                    }
                    Modal::ChatgptWelcome => {
                        if let Some(store) = &self.auth
                            && let Err(error) = store.welcome_seen()
                        {
                            self.status = error;
                        }
                        return;
                    }
                    _ => {}
                },
                _ => {}
            }
            self.modal = Some(modal);
        }
    }
    fn import_key(&mut self, key: KeyEvent, mut modal: Modal) {
        if let Modal::Import(query) = &mut modal {
            let entries = self.import_matches(query).unwrap_or_default();
            let count = entries.len();
            match key.code {
                KeyCode::Tab if count > 0 => self.modal_index = (self.modal_index + 1) % count,
                KeyCode::BackTab if count > 0 => {
                    self.modal_index = (self.modal_index + count - 1) % count
                }
                KeyCode::Up => self.modal_index = self.modal_index.saturating_sub(1),
                KeyCode::Down => {
                    self.modal_index = (self.modal_index + 1).min(count.saturating_sub(1))
                }
                KeyCode::Right | KeyCode::Enter => {
                    // An explicit file path remains valid even when it is hidden or unmatched.
                    let exact = self.source_path(std::path::Path::new(query.trim()));
                    if key.code == KeyCode::Enter && self.modal_index == 0 && exact.is_file() {
                        self.start_import(exact);
                        return;
                    }
                    if let Some(entry) = entries.get(self.modal_index.min(count.saturating_sub(1)))
                    {
                        if key.code == KeyCode::Enter && !entry.directory {
                            self.start_import(entry.path.clone());
                            return;
                        }
                        *query = entry.path.to_string_lossy().into_owned();
                        if entry.directory {
                            query.push(std::path::MAIN_SEPARATOR);
                        }
                        self.modal_index = 0;
                    } else if key.code == KeyCode::Enter {
                        self.status = "No matching source file".into();
                    }
                }
                KeyCode::Backspace => {
                    query.pop();
                    self.modal_index = 0;
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    query.clear();
                    self.modal_index = 0;
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    query.push(c);
                    self.modal_index = 0;
                }
                _ => {}
            }
            self.modal = Some(modal);
        }
    }
    fn ex_key(&mut self, key: KeyEvent, mut modal: Modal) {
        if let Modal::Ex(buffer) = &mut modal {
            if matches!(key.code, KeyCode::Tab | KeyCode::BackTab)
                && self.ex_completion_prefix.is_none()
                && buffer.trim() == "theme"
            {
                *buffer = "theme ".into();
                self.ex_selected = None;
            }
            let matches = self.ex_menu_suggestions(buffer);
            match key.code {
                KeyCode::Enter => {
                    self.cancel_theme_preview();
                    let mut line = buffer.trim().to_owned();
                    if let Some(suggestion) =
                        self.ex_selected.and_then(|selected| matches.get(selected))
                    {
                        line = suggestion.completion.clone();
                    }
                    if !line.is_empty() {
                        if self.ex_history.last() != Some(&line) {
                            self.ex_history.push(line.clone());
                        }
                        self.ex_selected = None;
                        self.ex_completion_prefix = None;
                        self.execute_ex(&line);
                    }
                    return;
                }
                KeyCode::Backspace if buffer.is_empty() => {
                    self.cancel_theme_preview();
                    return;
                }
                KeyCode::Backspace => {
                    self.cancel_theme_preview();
                    buffer.pop();
                    self.ex_selected = None;
                    self.ex_completion_prefix = None;
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.cancel_theme_preview();
                    buffer.clear();
                    self.ex_selected = None;
                    self.ex_completion_prefix = None;
                }
                KeyCode::Up | KeyCode::Down if key.modifiers.contains(KeyModifiers::ALT) => {
                    self.cancel_theme_preview();
                    self.ex_history_index = if key.code == KeyCode::Up {
                        self.ex_history_index.saturating_sub(1)
                    } else {
                        (self.ex_history_index + 1).min(self.ex_history.len())
                    };
                    *buffer = self
                        .ex_history
                        .get(self.ex_history_index)
                        .cloned()
                        .unwrap_or_default();
                    self.ex_selected = None;
                    self.ex_completion_prefix = None;
                }
                KeyCode::Up if !matches.is_empty() => {
                    self.ex_selected = Some(
                        self.ex_selected
                            .map(|i| if i == 0 { matches.len() - 1 } else { i - 1 })
                            .unwrap_or(matches.len() - 1),
                    );
                    if self.ex_completion_prefix.is_some() {
                        *buffer = matches[self.ex_selected.unwrap()].completion.clone();
                    }
                }
                KeyCode::Down if !matches.is_empty() => {
                    self.ex_selected = Some(
                        self.ex_selected
                            .map(|i| (i + 1) % matches.len())
                            .unwrap_or(0),
                    );
                    if self.ex_completion_prefix.is_some() {
                        *buffer = matches[self.ex_selected.unwrap()].completion.clone();
                    }
                }
                KeyCode::Tab | KeyCode::BackTab if !matches.is_empty() => {
                    let cycling = self.ex_completion_prefix.is_some();
                    let backwards = key.code == KeyCode::BackTab;
                    let selected = match self.ex_selected {
                        Some(index) if cycling && backwards => {
                            (index + matches.len() - 1) % matches.len()
                        }
                        Some(index) if cycling => (index + 1) % matches.len(),
                        Some(index) => index,
                        None if backwards => matches.len() - 1,
                        None => 0,
                    };
                    if !cycling {
                        self.ex_completion_prefix = Some(buffer.clone());
                    }
                    self.ex_selected = Some(selected);
                    *buffer = matches[selected].completion.clone();
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    self.cancel_theme_preview();
                    buffer.push(c);
                    self.ex_selected = None;
                    self.ex_completion_prefix = None;
                }
                _ => {}
            }
            if matches!(
                key.code,
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Up | KeyCode::Down
            ) && !key.modifiers.contains(KeyModifiers::ALT)
                && let Some(suggestion) = self.ex_selected.and_then(|i| matches.get(i))
                && let Some(name) = suggestion.completion.strip_prefix("theme ")
            {
                self.preview_theme(name);
            }
            self.modal = Some(modal);
        }
    }
    fn notebook_picker_key(&mut self, key: KeyEvent, mut modal: Modal) {
        if let Modal::SelectNotebook(query) = &mut modal {
            let matches = self.notebook_matches(query);
            self.modal_index = self.modal_index.min(matches.len().saturating_sub(1));
            match key.code {
                KeyCode::Up => self.modal_index = self.modal_index.saturating_sub(1),
                KeyCode::Down => {
                    self.modal_index = (self.modal_index + 1).min(matches.len().saturating_sub(1))
                }
                KeyCode::Tab if !matches.is_empty() => {
                    self.modal_index = (self.modal_index + 1) % matches.len();
                }
                KeyCode::BackTab if !matches.is_empty() => {
                    self.modal_index = (self.modal_index + matches.len() - 1) % matches.len();
                }
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if let Some(&notebook) = matches.get(self.modal_index) {
                        let selected = self.modal_index;
                        self.modal = Some(Modal::DeleteNotebook {
                            notebook,
                            query: query.clone(),
                            selected,
                        });
                        self.modal_index = 0;
                        return;
                    }
                }
                KeyCode::Enter => {
                    if let Some(index) = matches.get(self.modal_index) {
                        self.action(Action::Notebook(*index));
                        return;
                    }
                }
                KeyCode::Backspace => {
                    query.pop();
                    self.modal_index = 0;
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    query.push(c);
                    self.modal_index = 0;
                }
                _ => {}
            }
            self.modal = Some(modal);
        }
    }
    fn reader_key(&mut self, key: KeyEvent, modal: Modal) {
        if matches!(modal, Modal::Reader) {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.modal_index = self.modal_index.saturating_sub(1)
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.modal_index = self.modal_index.saturating_add(1)
                }
                KeyCode::PageUp => self.modal_index = self.modal_index.saturating_sub(10),
                KeyCode::PageDown => self.modal_index = self.modal_index.saturating_add(10),
                KeyCode::Home => self.modal_index = 0,
                KeyCode::End => self.modal_index = usize::MAX,
                _ => {}
            }
            self.modal = Some(modal);
        }
    }
    fn help_key(&mut self, key: KeyEvent, modal: Modal) {
        if matches!(modal, Modal::Help) {
            match key.code {
                KeyCode::Down | KeyCode::PageDown => {
                    self.modal_index = (self.modal_index + 1).min(64);
                    self.modal = Some(modal);
                }
                KeyCode::Up | KeyCode::PageUp => {
                    self.modal_index = self.modal_index.saturating_sub(1);
                    self.modal = Some(modal);
                }
                _ => {}
            }
        }
    }
    fn leader_key(&mut self, key: KeyEvent, modal: Modal) {
        if matches!(modal, Modal::Leader) {
            if key.kind == KeyEventKind::Repeat {
                self.modal = Some(modal);
                return;
            }
            if matches!(
                key.code,
                KeyCode::Up | KeyCode::PageUp | KeyCode::Down | KeyCode::PageDown
            ) {
                self.modal_index = if matches!(key.code, KeyCode::Up | KeyCode::PageUp) {
                    self.modal_index.saturating_sub(1)
                } else {
                    (self.modal_index + 1).min(12)
                };
                self.modal = Some(modal);
                return;
            }
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                && let KeyCode::Char(c) = key.code
            {
                self.leader_command(c);
            }
        }
    }
    fn card_key(&mut self, key: KeyEvent, mut modal: Modal) {
        if let Modal::Card {
            id,
            front,
            back,
            field,
            citation,
        } = &mut modal
        {
            let save =
                key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s');
            let approve =
                key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('a');
            if save || approve {
                if front.trim().is_empty() || back.trim().is_empty() {
                    self.status = "Both a question and an answer are required.".into();
                    self.modal = Some(modal);
                    return;
                }
                if let Some(id) = id {
                    if let Some(card) = self
                        .library
                        .book_mut()
                        .cards
                        .iter_mut()
                        .find(|c| c.id == *id)
                    {
                        card.front = front.trim().into();
                        card.back = back.trim().into();
                        if approve {
                            card.draft = false;
                        }
                    }
                } else {
                    let id = self.library.id();
                    self.library.book_mut().cards.push(Card {
                        id,
                        front: front.trim().into(),
                        back: back.trim().into(),
                        citation: citation.clone(),
                        draft: !approve,
                        history: vec![],
                    });
                }
                self.dirty = true;
                self.status = if approve {
                    "Card saved."
                } else {
                    "Card saved. Select it from the Flashcards list."
                }
                .into();
                self.card = self.card.min(self.cards().len().saturating_sub(1));
                return;
            }
            match key.code {
                KeyCode::Tab | KeyCode::BackTab => *field = 1 - *field,
                KeyCode::Backspace => {
                    if *field == 0 {
                        front.pop();
                    } else {
                        back.pop();
                    }
                }
                KeyCode::Enter => {
                    if *field == 0 {
                        *field = 1;
                    } else {
                        back.push('\n');
                    }
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if *field == 0 {
                        front.clear();
                    } else {
                        back.clear();
                    }
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    if *field == 0 {
                        front.push(c);
                    } else {
                        back.push(c);
                    }
                }
                _ => {}
            }
            self.modal = Some(modal);
        }
    }
    fn text_key(&mut self, key: KeyEvent, mut modal: Modal) {
        let count = if matches!(&modal, Modal::Palette(_)) {
            let query = match &modal {
                Modal::Palette(s) => s.to_lowercase(),
                _ => unreachable!(),
            };
            COMMANDS
                .iter()
                .filter(|c| c.label().to_lowercase().contains(&query))
                .count()
        } else if let Modal::Search(query) = &modal {
            self.search(query).len()
        } else {
            0
        };
        let text = match &mut modal {
            Modal::Import(s) | Modal::Notebook(s) | Modal::Search(s) | Modal::Palette(s) => s,
            _ => unreachable!(),
        };
        match key.code {
            KeyCode::Up => self.modal_index = self.modal_index.saturating_sub(1),
            KeyCode::Down => self.modal_index = (self.modal_index + 1).min(count.saturating_sub(1)),
            KeyCode::Tab if count > 0 => self.modal_index = (self.modal_index + 1) % count,
            KeyCode::BackTab if count > 0 => {
                self.modal_index = (self.modal_index + count - 1) % count;
            }
            KeyCode::Backspace => {
                text.pop();
                self.modal_index = 0;
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                text.clear();
                self.modal_index = 0;
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                text.push(c);
                self.modal_index = 0;
            }
            KeyCode::Enter => {
                let value = text.trim().to_owned();
                match &modal {
                    Modal::Import(_) => self.start_import(PathBuf::from(&value)),
                    Modal::Notebook(_) if !value.is_empty() => {
                        self.create_notebook(value);
                        self.status = format!(
                            "Notebook created · {} a to import a source",
                            self.leader_label()
                        );
                    }
                    Modal::Palette(_) => {
                        if let Some((index, _)) = COMMANDS
                            .iter()
                            .enumerate()
                            .filter(|(_, c)| {
                                c.label().to_lowercase().contains(&value.to_lowercase())
                            })
                            .nth(self.modal_index)
                        {
                            self.command(COMMANDS[index]);
                        }
                    }
                    Modal::Search(_) => {
                        if let Some((_, action)) =
                            self.search(&value).get(self.modal_index).cloned()
                        {
                            self.action(action);
                        } else {
                            self.status = "No matches in this notebook.".into();
                        }
                    }
                    _ => {}
                }
                return;
            }
            _ => {}
        }
        self.modal = Some(modal);
    }
}
