use super::*;

#[test]
fn typing_never_triggers_navigation_shortcuts() {
    let mut app = selected_app();
    press(&mut app, KeyCode::Char('i'));
    for c in "a question about r and q 🦀".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    assert!(!app.quit);
    assert_eq!(app.input, "a question about r and q 🦀");
    press(&mut app, KeyCode::Backspace);
    assert_eq!(app.input, "a question about r and q ");
    press(&mut app, KeyCode::Esc);
    assert!(!app.editing);
}

#[test]
fn custom_leader_is_literal_while_typing_and_repeat_does_not_dispatch() {
    let mut app = selected_app();
    app.leader = ',';
    leader(&mut app, 'o');
    assert!(app.tab == Tab::Notes);
    press(&mut app, KeyCode::Char('i'));
    press(&mut app, KeyCode::Char(','));
    press(&mut app, KeyCode::Char(' '));
    assert!(app.library.book().notes.starts_with(", "));
    assert!(app.modal.is_none());
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char(','));
    app.key(KeyEvent {
        kind: KeyEventKind::Repeat,
        ..KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::SHIFT)
    });
    assert!(!app.quit);
    assert!(matches!(app.modal, Some(Modal::Leader)));
    press(&mut app, KeyCode::Char('!'));
    assert!(app.modal.is_none());
    assert!(!app.quit);
}

#[test]
fn notes_edit_at_the_cursor_preview_without_changing_markdown_and_keep_reader_separate() {
    let mut app = selected_app();
    app.library.book_mut().notes = "# Topic\n\n**héllo**".into();
    app.open(Tab::Notes);
    press(&mut app, KeyCode::Char('i'));
    app.paste("New ");
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.library.book().notes, "New # Topic\n\n**héllo**");
    assert!(app.dirty);
    press(&mut app, KeyCode::Char('u'));
    assert_eq!(app.library.book().notes, "# Topic\n\n**héllo**");
    leader(&mut app, 'p');
    assert!(app.notes.preview);
    let before = app.library.book().notes.clone();
    app.action(Action::ReadSource(0));
    assert!(matches!(app.modal, Some(Modal::Reader)));
    assert!(app.tab == Tab::Notes);
    press(&mut app, KeyCode::Esc);
    assert!(app.modal.is_none());
    assert_eq!(app.library.book().notes, before);
    app.action(Action::Notebook(1));
    app.open(Tab::Notes);
    press(&mut app, KeyCode::Char('i'));
    press(&mut app, KeyCode::Char('u'));
    assert_eq!(app.library.book().notes, "u");
}

#[test]
fn palette_selection_and_search_open_the_chosen_destination() {
    let mut app = selected_app();
    ctrl(&mut app, 'k');
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert!(app.tab == Tab::Cards);
    app.modal = Some(Modal::Search("replication".into()));
    app.modal_index = 0;
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.modal, Some(Modal::Reader)));
    assert_eq!(app.source, 0);
}

#[test]
fn menu_tabs_wrap_preserve_filters_and_wait_for_confirmation() {
    let mut app = selected_app();
    app.pick_notebook();
    press(&mut app, KeyCode::BackTab);
    assert_eq!(app.modal_index, app.library.notebooks.len() - 1);
    assert_eq!(app.library.active, 0);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.modal_index, 0);
    for c in "Biology".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::BackTab);
    assert_eq!(app.modal_index, 0);
    assert!(matches!(&app.modal, Some(Modal::SelectNotebook(q)) if q == "Biology"));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.library.active, 1);

    app.action(Action::Notebook(0));
    for modal in [
        Modal::Palette("card".into()),
        Modal::Search("replication".into()),
    ] {
        let count = match &modal {
            Modal::Palette(q) => COMMANDS
                .iter()
                .filter(|c| c.label().to_lowercase().contains(q))
                .count(),
            Modal::Search(q) => app.search(q).len(),
            _ => unreachable!(),
        };
        assert!(count > 0);
        app.modal = Some(modal);
        app.modal_index = 0;
        press(&mut app, KeyCode::BackTab);
        assert_eq!(app.modal_index, count - 1);
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.modal_index, 0);
        assert!(app.modal.is_some());
    }
    for modal in [
        Modal::SelectNotebook("no matches".into()),
        Modal::Palette("no matches".into()),
        Modal::Search("no matches".into()),
    ] {
        app.modal = Some(modal);
        app.modal_index = 0;
        press(&mut app, KeyCode::Tab);
        press(&mut app, KeyCode::BackTab);
        assert_eq!(app.modal_index, 0);
        assert!(app.modal.is_some());
    }
}

#[test]
fn colon_commands_complete_recall_history_and_leave_typing_alone() {
    let mut app = selected_app();
    press(&mut app, KeyCode::Char(':'));
    for c in "car".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    press(&mut app, KeyCode::Tab);
    assert!(matches!(&app.modal, Some(Modal::Ex(s)) if s == "cards"));
    press(&mut app, KeyCode::Enter);
    assert!(app.tab == Tab::Cards);
    press(&mut app, KeyCode::Char(':'));
    app.key(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
    assert!(matches!(&app.modal, Some(Modal::Ex(s)) if s == "cards"));
    press(&mut app, KeyCode::Esc);
    ex(&mut app, "w");
    assert!(app.save_requested);
    ex(&mut app, "not-a-command");
    assert!(app.status.starts_with("Unknown command"));
    assert!(!app.quit);
    app.open(Tab::Chat);
    press(&mut app, KeyCode::Char('i'));
    for c in ":q".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    assert_eq!(app.input, ":q");
    assert!(app.modal.is_none());
    assert!(!app.quit);
}

#[test]
fn command_menu_selection_completes_and_runs_the_chosen_command() {
    let mut app = selected_app();
    press(&mut app, KeyCode::Char(':'));
    press(&mut app, KeyCode::Enter);
    assert!(!app.quit); // Merely opening an empty menu never selects Quit.
    press(&mut app, KeyCode::Char(':'));
    for c in "car".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    assert_eq!(ex_matches("car").len(), 2);
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Down); // card, rather than cards
    press(&mut app, KeyCode::Tab);
    assert!(matches!(&app.modal, Some(Modal::Ex(s)) if s == "card"));
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.modal, Some(Modal::Card { .. })));
    assert_eq!(EX_COMMANDS[ex_matches("qu")[0]].0, "q");
    assert_eq!(ex_matches("notebook Biology").len(), 1);
    assert!(ex_matches("nonexistent").is_empty());
}
