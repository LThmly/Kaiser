//! Named command registries and colon, leader, and palette dispatch.
use super::{
    Action, App, EditorTarget, Focus, JobKind, KeyCode, KeyEvent, KeyModifiers, Modal, Provider,
    Tab,
};

/// Named application actions shared by the palette, shortcuts, and colon commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    OpenChat,
    OpenCard,
    BrowseCards,
    BrowseDrafts,
    OpenNotes,
    ReadSource,
    ImportSource,
    NewNotebook,
    DraftCard,
    ToggleSidebar,
    MaximizeWorkspace,
    Help,
    Quit,
    SelectNotebook,
    Accounts,
    GenerateCards,
}
impl Command {
    pub fn label(self) -> &'static str {
        match self {
            Self::OpenChat => "Open chat",
            Self::OpenCard => "Open flashcard",
            Self::BrowseCards => "Browse cards",
            Self::BrowseDrafts => "Browse drafts",
            Self::OpenNotes => "Open notes",
            Self::ReadSource => "Read selected source",
            Self::ImportSource => "Import source",
            Self::NewNotebook => "New notebook",
            Self::DraftCard => "Draft card from conversation",
            Self::ToggleSidebar => "Toggle sidebar",
            Self::MaximizeWorkspace => "Maximize workspace",
            Self::Help => "Help",
            Self::Quit => "Quit",
            Self::SelectNotebook => "Select notebook",
            Self::Accounts => "Continue with ChatGPT",
            Self::GenerateCards => "Generate flashcards from sources",
        }
    }
}

pub const COMMANDS: &[Command] = &[
    Command::OpenChat,
    Command::OpenCard,
    Command::BrowseCards,
    Command::BrowseDrafts,
    Command::OpenNotes,
    Command::ReadSource,
    Command::ImportSource,
    Command::NewNotebook,
    Command::DraftCard,
    Command::ToggleSidebar,
    Command::MaximizeWorkspace,
    Command::Help,
    Command::Quit,
    Command::SelectNotebook,
    Command::Accounts,
    Command::GenerateCards,
];

pub const LEADER_BINDINGS: &[(char, &str)] = &[
    ('h', "Home"),
    ('c', "Chat"),
    ('f', "Select notebook"),
    ('F', "Flashcards"),
    ('n', "New notebook"),
    ('o', "Notes"),
    ('s', "Sources"),
    ('w', "Workspace"),
    ('e', "Flashcards panel"),
    ('d', "Drafts"),
    ('a', "Import source"),
    ('m', "Make card"),
    ('g', "Generate flashcards"),
    ('/', "Search"),
    ('p', "Notes preview / editor"),
    ('P', "Commands"),
    ('z', "Maximize"),
    ('B', "Toggle sidebar"),
    ('?', "Help"),
    ('Q', "Save and quit"),
];

pub const EX_COMMANDS: &[(&str, &str)] = &[
    ("q", "Save and quit · alias: quit"),
    ("w", "Save library · alias: write"),
    ("wq", "Save and quit · alias: x"),
    ("notebook", "Select notebook [name]"),
    ("new", "Create notebook [name]"),
    ("chat", "Open chat"),
    ("clear", "Clear this notebook’s chat"),
    (
        "generate-cards",
        "Generate flashcard drafts [count] [scope]",
    ),
    ("cards", "Browse flashcards"),
    ("notes", "Open notes"),
    ("notes-preview", "Toggle rendered Markdown and notes editor"),
    ("sources", "Focus source selection"),
    ("read", "Read selected source"),
    ("drafts", "Browse card drafts"),
    ("card", "Draft a card from the conversation"),
    ("import", "Import source [path]"),
    ("help", "Show the keybinding guide"),
    ("config-open", "Edit the active config file"),
    ("config-reload", "Reload config and configured theme"),
    ("theme", "Choose a theme [name]"),
    ("theme-open", "Edit the current theme file"),
    ("theme-reload", "Reload current theme colors"),
    ("cd", "Set this notebook’s working directory [path]"),
    ("pwd", "Show this notebook’s working directory"),
    ("login", "Continue with ChatGPT"),
    ("account", "Choose a ChatGPT account"),
    ("logout", "Sign out of ChatGPT"),
    ("model", "Choose a ChatGPT model"),
    ("usage", "Manage ChatGPT plan usage"),
    ("ai-usage", "Show local AI request and token usage"),
    (
        "ai-resume",
        "Resume AI requests after reviewing a provider limit",
    ),
    ("ai-cache-clear", "Clear cached AI answers"),
];

pub struct ExSuggestion {
    pub completion: String,
    pub label: String,
    pub description: String,
}

pub fn ex_matches(input: &str) -> Vec<usize> {
    let input = input.trim_start();
    let prefix = input.split_whitespace().next().unwrap_or("");
    let arguments = input.chars().any(char::is_whitespace);
    EX_COMMANDS
        .iter()
        .enumerate()
        .filter(|(_, (name, _))| {
            let alias = match *name {
                "q" => "quit",
                "w" => "write",
                "wq" => "x",
                _ => name,
            };
            if arguments {
                *name == prefix || alias == prefix
            } else {
                name.starts_with(prefix) || alias.starts_with(prefix)
            }
        })
        .map(|(index, _)| index)
        .collect()
}

impl App {
    pub(super) fn leader_command(&mut self, key: char) {
        if !self.notebook_selected
            && matches!(
                key,
                'c' | 'F' | 'r' | 'o' | 's' | 'e' | 'd' | 'a' | 'm' | '/' | 'p'
            )
        {
            self.pick_notebook();
            return;
        }
        match key {
            'h' => self.go_home(),
            'c' => self.open(Tab::Chat),
            'f' => self.pick_notebook(),
            'F' => {
                self.drafts_only = false;
                self.card = 0;
                self.open(Tab::Cards);
            }
            'r' => self.open(Tab::Cards),
            'o' => self.open(Tab::Notes),
            's' => {
                self.sidebar = true;
                self.maximized = false;
                self.focus = Focus::Sources;
            }
            'w' => self.focus = Focus::Workspace,
            'e' => {
                self.maximized = false;
                self.focus = Focus::Flashcards;
            }
            'd' => {
                self.drafts_only = true;
                self.card = 0;
                self.open(Tab::Cards);
            }
            'a' => {
                self.modal_index = 0;
                self.modal = Some(Modal::Import(String::new()));
            }
            'n' | 'N' => self.new_notebook(),
            'm' => self.draft(),
            'g' => self.execute_ex("generate-cards"),
            '/' => {
                self.modal_index = 0;
                self.modal = Some(Modal::Search(String::new()));
            }
            'p' => self.toggle_notes_preview(),
            'P' => {
                self.modal_index = 0;
                self.modal = Some(Modal::Palette(String::new()));
            }
            'z' => self.maximized = !self.maximized,
            'B' => self.sidebar = !self.sidebar,
            '?' => {
                self.modal_index = 0;
                self.modal = Some(Modal::Help);
            }
            'Q' => self.quit = true,
            _ => {
                self.status = format!(
                    "No binding for '{key}' · {} opens navigation · Esc cancels",
                    self.leader_label()
                )
            }
        }
    }
    pub fn command(&mut self, command: Command) {
        if !self.notebook_selected
            && matches!(
                command,
                Command::OpenChat
                    | Command::OpenCard
                    | Command::BrowseCards
                    | Command::BrowseDrafts
                    | Command::OpenNotes
                    | Command::ReadSource
                    | Command::ImportSource
                    | Command::DraftCard
            )
        {
            self.pick_notebook();
            return;
        }
        match command {
            Command::OpenChat => self.open(Tab::Chat),
            Command::OpenCard => self.open(Tab::Cards),
            Command::BrowseCards | Command::BrowseDrafts => {
                self.drafts_only = command == Command::BrowseDrafts;
                self.card = 0;
                self.open(Tab::Cards);
            }
            Command::OpenNotes => self.open(Tab::Notes),
            Command::ReadSource => self.read_source(),
            Command::ImportSource => {
                self.modal_index = 0;
                self.modal = Some(Modal::Import(String::new()));
            }
            Command::NewNotebook => self.new_notebook(),
            Command::DraftCard => self.draft(),
            Command::ToggleSidebar => self.sidebar = !self.sidebar,
            Command::MaximizeWorkspace => self.maximized = !self.maximized,
            Command::Help => {
                self.modal_index = 0;
                self.modal = Some(Modal::Help);
            }
            Command::Quit => self.quit = true,
            Command::SelectNotebook => self.pick_notebook(),
            Command::Accounts => self.account_menu(),
            Command::GenerateCards => self.execute_ex("generate-cards"),
        }
    }
    pub fn execute_ex(&mut self, line: &str) {
        let line = line.trim();
        let (name, argument) = line
            .split_once(char::is_whitespace)
            .map(|(name, rest)| (name, rest.trim()))
            .unwrap_or((line, ""));
        if !argument.is_empty()
            && !["notebook", "new", "import", "theme", "cd", "generate-cards"].contains(&name)
        {
            self.status = format!(":{name} does not take arguments. Try :help.");
            return;
        }
        match name {
            "ai-usage" => {
                self.modal_index = 0;
                let scope = self.provider.as_ref().map(Provider::scope);
                match self.ai.describe(scope.as_deref()) {
                    Ok(text) => self.modal = Some(Modal::AiUsage(text)),
                    Err(error) => self.status = error,
                }
            }
            "ai-resume" => {
                if let Some(provider) = &self.provider {
                    self.status = match self.ai.resume(&provider.scope()) {
                        Ok(()) => {
                            "AI provider pause cleared. The local daily limit still applies.".into()
                        }
                        Err(error) => error,
                    };
                }
            }
            "ai-cache-clear" => {
                self.ai.clear_cache();
                self.status = "Cached AI answers cleared".into();
            }
            "login" => {
                if let Some(connection) = &self.chatgpt {
                    self.start_login(Some(connection.client_id.clone()));
                } else if self.accounts.is_empty() {
                    self.start_login(None);
                } else {
                    self.account_menu();
                }
            }
            "account" => self.account_menu(),
            "logout" => self.sign_out(),
            "model" => self.load_models(),
            "usage" => {
                self.status = match crate::auth::open_browser(crate::auth::USAGE_URL) {
                    Ok(()) => "ChatGPT usage settings opened".into(),
                    Err(_) => format!("Manage ChatGPT usage: {}", crate::auth::USAGE_URL),
                };
            }
            "q" | "quit" | "wq" | "x" => self.quit = true,
            "w" | "write" => self.save_requested = true,
            "config-open" => self.open_config_requested = Some(EditorTarget::Config),
            "config-reload" => self.reload_config(),
            "theme-open" => self.open_config_requested = Some(EditorTarget::Theme),
            "theme-reload" => {
                let name = self.theme_name.clone();
                self.apply_theme(&name);
            }
            "theme" if argument.is_empty() => {
                if let Ok(themes) = crate::config::theme_names(&self.config_path) {
                    self.themes = themes;
                }
                self.ex_selected = None;
                self.ex_completion_prefix = None;
                self.modal = Some(Modal::Ex("theme ".into()));
            }
            "theme" => self.apply_theme(argument),
            "notebook" if argument.is_empty() => self.pick_notebook(),
            "notebook" => {
                if let Some(index) = self
                    .library
                    .notebooks
                    .iter()
                    .position(|book| book.name.eq_ignore_ascii_case(argument))
                {
                    self.action(Action::Notebook(index));
                } else {
                    self.status =
                        format!("Notebook '{argument}' not found. Use :notebook to choose one.");
                }
            }
            "new" if argument.is_empty() => self.command(Command::NewNotebook),
            "new" => self.create_notebook(argument.into()),
            "cd" => self.change_directory(argument),
            "pwd" => {
                if self.require_notebook() {
                    self.status = format!("Directory: {}", self.library.book().directory.display());
                }
            }
            "chat" => self.command(Command::OpenChat),
            "generate-cards" => match crate::flashcards::Request::parse(argument) {
                Ok(request) => self.generate_cards(request, None),
                Err(error) => self.status = error,
            },
            "clear" => {
                if !self.require_notebook() {
                    return;
                }
                if self.pending.as_ref().is_some_and(|job| {
                    job.notebook == self.library.active && matches!(job.kind, JobKind::Chat)
                }) {
                    self.cancel_generation();
                }
                self.library.book_mut().messages.clear();
                self.input.clear();
                self.scroll = None;
                self.dirty = true;
                self.status = "Chat cleared".into();
            }
            "cards" => self.command(Command::BrowseCards),
            "review" => self.command(Command::OpenCard),
            "notes" => self.command(Command::OpenNotes),
            "notes-preview" => self.toggle_notes_preview(),
            "drafts" => self.command(Command::BrowseDrafts),
            "read" => self.command(Command::ReadSource),
            "sources" => self.leader_command('s'),
            "card" => self.command(Command::DraftCard),
            "help" => self.command(Command::Help),
            "import" => {
                if !self.require_notebook() {
                    return;
                }
                self.modal_index = 0;
                self.modal = Some(Modal::Import(argument.into()));
                if !argument.is_empty() {
                    self.modal_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
                }
            }
            _ => self.status = format!("Unknown command :{line}. Try :help."),
        }
    }
}
