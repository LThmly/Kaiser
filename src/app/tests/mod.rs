//! Application behavior tests grouped by feature.
mod chat;
mod flashcards;
mod input;
mod notebooks;
mod settings;
mod sources;

use super::*;
use crate::model::Citation;

pub(super) fn selected_app() -> App {
    let mut app = App::new(Library::demo(), None);
    app.action(Action::Notebook(0));
    app
}

pub(super) fn press(app: &mut App, code: KeyCode) {
    app.key(KeyEvent::new(code, KeyModifiers::NONE));
}

pub(super) fn leader(app: &mut App, key: char) {
    let prefix = app.leader;
    press(app, KeyCode::Char(prefix));
    press(app, KeyCode::Char(key));
}

pub(super) fn ctrl(app: &mut App, c: char) {
    app.key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
}

pub(super) fn ex(app: &mut App, command: &str) {
    press(app, KeyCode::Char(':'));
    for c in command.chars() {
        press(app, KeyCode::Char(c));
    }
    press(app, KeyCode::Enter);
}
