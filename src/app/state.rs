//! Shared navigation, modal, and background-job types.
use super::Command;
use crate::model::Citation;
use std::sync::mpsc;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Notebooks,
    Sources,
    Workspace,
    Flashcards,
}
impl Focus {
    pub fn next(self, backwards: bool) -> Self {
        let all = [
            Self::Notebooks,
            Self::Sources,
            Self::Workspace,
            Self::Flashcards,
        ];
        let at = all.iter().position(|p| *p == self).unwrap();
        all[(at + if backwards { 3 } else { 1 }) % 4]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Chat,
    Cards,
    Notes,
}
#[derive(Clone)]
pub enum Modal {
    SummaryPlan {
        cards: Option<crate::flashcards::Request>,
        fingerprint: String,
        requests: usize,
        label: String,
        bytes: usize,
    },
    AiUsage(String),
    Accounts,
    ChatgptModels,
    SignIn(String),
    ChatgptWelcome,
    Help,
    Reader,
    Leader,
    Ex(String),
    SelectNotebook(String),
    DeleteNotebook {
        notebook: usize,
        query: String,
        selected: usize,
    },
    Import(String),
    Notebook(String),
    Search(String),
    Palette(String),
    Card {
        id: Option<u64>,
        front: String,
        back: String,
        field: usize,
        citation: Option<Citation>,
    },
}

#[derive(Clone, Copy)]
pub enum Action {
    ConfirmSummary(bool),
    ComposeChat,
    SendChat,
    Focus(Focus),
    Notebook(usize),
    Source(usize),
    PickNotebook,
    ConfirmNotebookDeletion(bool),
    Command(Command),
    Tab(Tab),
    ReadSource(usize),
    OpenCard(usize),
    SelectFlashcard(usize),
}

pub struct PendingImport {
    pub receiver: mpsc::Receiver<Result<crate::import::ExtractedSource, String>>,
    pub notebook: usize,
}

#[derive(Clone, Copy)]
pub enum JobKind {
    Chat,
    Cards(usize),
}

pub struct Pending {
    pub kind: JobKind,
    pub receiver: mpsc::Receiver<Result<String, String>>,
    pub notebook: usize,
    pub citations: Vec<Citation>,
}

pub enum AccountTaskResult {
    Models(Vec<crate::auth::Model>),
    SignedOut(String),
}

#[derive(Clone, Copy)]
pub enum EditorTarget {
    Config,
    Theme,
}
