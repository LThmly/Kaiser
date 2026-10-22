use super::*;

#[test]
fn chatgpt_account_and_model_menus_cycle_and_do_not_require_a_notebook() {
    let root = std::env::temp_dir().join(format!("kaiser-account-ui-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let record = |id: &str| {
        serde_json::json!({
            "client_id": id, "subject": id, "email": "same@example.invalid",
            "access_token": "synthetic", "refresh_token": "", "id_token": "synthetic",
            "scopes": ["chatgpt.tokens.use.direct"], "expires_at": 4102444800_u64,
            "model": "first-model"
        })
    };
    std::fs::write(
        root.join("accounts.json"),
        serde_json::to_vec(&serde_json::json!({
            "host": "synthetic-host", "active": "oaiapp_first",
            "accounts": [record("oaiapp_first"), record("oaiapp_second")]
        }))
        .unwrap(),
    )
    .unwrap();
    let mut app = App::new(Library::demo(), None);
    app.initialize_auth(crate::auth::Store::new(root.clone()));
    assert!(app.provider.as_ref().unwrap().uses_chatgpt());
    app.execute_ex("account");
    assert!(matches!(app.modal, Some(Modal::Accounts)));
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.chatgpt.as_ref().unwrap().client_id, "oaiapp_second");
    assert!(!app.notebook_selected);
    app.execute_ex("account");
    press(&mut app, KeyCode::BackTab);
    assert_eq!(app.modal_index, 2);
    press(&mut app, KeyCode::Esc);
    app.chatgpt_models = vec![crate::auth::Model {
        slug: "second-model".into(),
        name: "Second model".into(),
    }];
    app.modal = Some(Modal::ChatgptModels);
    app.modal_index = 0;
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.provider.as_ref().unwrap().model, "second-model");
    assert_eq!(
        app.auth
            .as_ref()
            .unwrap()
            .current()
            .unwrap()
            .unwrap()
            .model
            .as_deref(),
        Some("second-model")
    );
    app.modal = Some(Modal::ChatgptWelcome);
    press(&mut app, KeyCode::Enter);
    assert!(app.modal.is_none());
    assert!(
        app.ex_suggestions("log")
            .iter()
            .any(|s| s.completion == "login")
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn leader_navigation_cancels_and_does_not_reveal_or_toggle_sources() {
    let mut app = selected_app();
    app.focus = Focus::Sources;
    let selected = app.library.book().sources[0].selected;
    press(&mut app, KeyCode::Char(' '));
    assert!(matches!(app.modal, Some(Modal::Leader)));
    assert_eq!(app.library.book().sources[0].selected, selected);
    press(&mut app, KeyCode::Esc);
    assert!(app.modal.is_none());
    assert!(app.focus == Focus::Sources);
    press(&mut app, KeyCode::Char('x'));
    assert_ne!(app.library.book().sources[0].selected, selected);
    leader(&mut app, 'r');
    assert!(app.tab == Tab::Cards);
    assert!(!app.revealed);
    press(&mut app, KeyCode::Char(' '));
    assert!(!app.revealed);
    press(&mut app, KeyCode::Char('c'));
    assert!(app.tab == Tab::Chat);
    assert!(app.modal.is_none());
    press(&mut app, KeyCode::Char('r'));
    assert!(app.tab == Tab::Chat);
}

#[test]
fn asynchronous_reply_stays_in_original_notebook() {
    let mut app = selected_app();
    let (tx, rx) = mpsc::channel();
    app.pending = Some(Pending {
        kind: JobKind::Chat,
        receiver: rx,
        notebook: 0,
        citations: vec![],
    });
    app.action(Action::Notebook(1));
    tx.send(Ok("An answer for distributed systems".into()))
        .unwrap();
    app.poll_answer();
    assert!(app.library.notebooks[1].messages.is_empty());
    assert_eq!(
        app.library.notebooks[0].messages.last().unwrap().text,
        "An answer for distributed systems"
    );
}

#[test]
fn startup_requires_explicit_notebook_selection_and_picker_can_cancel() {
    let mut library = Library::demo();
    library.active = 1;
    let mut app = App::new(library, None);
    assert!(!app.notebook_selected);
    app.key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::SUPER));
    assert!(matches!(app.modal, Some(Modal::SelectNotebook(_))));
    press(&mut app, KeyCode::Down);
    assert_eq!(app.library.active, 1);
    press(&mut app, KeyCode::Esc);
    assert!(!app.notebook_selected);
    assert!(app.modal.is_none());
    app.key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::SUPER));
    for c in "Distributed".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    assert!(!app.notebook_selected);
    press(&mut app, KeyCode::Enter);
    assert!(app.notebook_selected);
    assert_eq!(app.library.active, 0);
    assert!(app.modal.is_none());
    let restarted = App::new(app.library.clone(), None);
    assert!(!restarted.notebook_selected);
}

#[test]
fn picker_deletes_filtered_notebooks_only_after_confirmation_and_preserves_the_active_one() {
    let mut app = selected_app();
    app.modal = Some(Modal::SelectNotebook("Biology".into()));
    app.modal_index = 0;
    ctrl(&mut app, 'd');
    assert!(matches!(
        &app.modal,
        Some(Modal::DeleteNotebook { notebook: 1, .. })
    ));
    press(&mut app, KeyCode::Enter); // Keep is selected by default.
    assert_eq!(app.library.notebooks.len(), 3);
    assert!(matches!(&app.modal, Some(Modal::SelectNotebook(query)) if query == "Biology"));
    ctrl(&mut app, 'd');
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.library.notebooks.len(), 3);
    ctrl(&mut app, 'd');
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.library.notebooks.len(), 2);
    assert_eq!(app.library.book().name, "Distributed systems");
    assert!(app.notebook_selected);
    assert!(matches!(&app.modal, Some(Modal::SelectNotebook(query)) if query.is_empty()));
    assert!(app.dirty);
}

#[test]
fn deletion_rebases_pending_jobs_and_discards_results_for_deleted_notebooks() {
    let mut app = selected_app();
    let (answer_tx, answer_rx) = mpsc::channel();
    let (import_tx, import_rx) = mpsc::channel();
    app.pending = Some(Pending {
        kind: JobKind::Chat,
        receiver: answer_rx,
        notebook: 2,
        citations: vec![],
    });
    app.pending_import = Some(PendingImport {
        receiver: import_rx,
        notebook: 2,
    });
    app.delete_notebook(1);
    assert_eq!(app.pending.as_ref().unwrap().notebook, 1);
    assert_eq!(app.pending_import.as_ref().unwrap().notebook, 1);
    answer_tx.send(Ok("Reading room reply".into())).unwrap();
    import_tx
        .send(Ok(crate::import::ExtractedSource {
            name: "Reading.txt".into(),
            content: "Reading room source".into(),
        }))
        .unwrap();
    app.poll_answer();
    app.poll_import();
    assert_eq!(
        app.library.notebooks[1].messages[0].text,
        "Reading room reply"
    );
    assert_eq!(app.library.notebooks[1].sources[0].name, "Reading.txt");
    let (answer_tx, answer_rx) = mpsc::channel();
    let (import_tx, import_rx) = mpsc::channel();
    app.pending = Some(Pending {
        kind: JobKind::Chat,
        receiver: answer_rx,
        notebook: 0,
        citations: vec![],
    });
    app.pending_import = Some(PendingImport {
        receiver: import_rx,
        notebook: 0,
    });
    crate::progress::record(&mut app.library, 1, 4);
    app.delete_notebook(0);
    assert!(app.pending.is_none());
    assert!(app.pending_import.is_none());
    assert!(answer_tx.send(Ok("late reply".into())).is_err());
    assert!(import_tx.send(Err("late import".into())).is_err());
    assert!(app.library.review_activity.is_empty());
    assert!(!app.notebook_selected);
    assert_eq!(app.library.notebooks[0].name, "Reading room");
}

#[test]
fn deleting_the_last_notebook_persists_an_empty_library_and_allows_creating_one() {
    let mut app = selected_app();
    while !app.library.notebooks.is_empty() {
        app.delete_notebook(0);
    }
    assert!(!app.notebook_selected);
    assert_eq!(app.library.active, 0);
    let path = std::env::temp_dir().join(format!(
        "kaiser-empty-library-{}/state.json",
        std::process::id()
    ));
    app.library.save(&path).unwrap();
    let saved = Library::load(&path).unwrap();
    assert!(saved.notebooks.is_empty());
    let mut reopened = App::new(saved, None);
    reopened.create_notebook("Fresh notebook".into());
    assert_eq!(reopened.library.book().name, "Fresh notebook");
    assert!(reopened.notebook_selected);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn super_f_opens_picker_while_typing_without_inserting_text() {
    let mut app = selected_app();
    app.editing = true;
    app.input = "Keep my question".into();
    app.key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::SUPER));
    assert!(matches!(app.modal, Some(Modal::SelectNotebook(_))));
    assert!(!app.editing);
    assert_eq!(app.input, "Keep my question");
    press(&mut app, KeyCode::Esc);
    leader(&mut app, 'm');
    assert!(matches!(app.modal, Some(Modal::Card { .. })));
    app.key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::SUPER));
    assert!(matches!(app.modal, Some(Modal::SelectNotebook(_))));
}

#[test]
fn super_f_switches_notebooks_from_every_workspace_and_menu() {
    let mut app = selected_app();
    for tab in [Tab::Chat, Tab::Cards, Tab::Notes] {
        for modal in [
            None,
            Some(Modal::Leader),
            Some(Modal::Help),
            Some(Modal::Ex("theme".into())),
            Some(Modal::Search("replication".into())),
            Some(Modal::Palette(String::new())),
            Some(Modal::Import(String::new())),
            Some(Modal::Notebook(String::new())),
            Some(Modal::Card {
                id: None,
                front: "Question".into(),
                back: "Answer".into(),
                field: 0,
                citation: None,
            }),
        ] {
            app.action(Action::Notebook(0));
            app.open(tab);
            app.modal = modal;
            app.key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::SUPER));
            assert!(matches!(app.modal, Some(Modal::SelectNotebook(_))));
            press(&mut app, KeyCode::Tab);
            press(&mut app, KeyCode::Enter);
            assert_eq!(app.library.active, 1);
            assert!(app.notebook_selected);
            assert!(app.modal.is_none());
        }
    }
}

#[test]
fn leader_n_creates_and_leader_f_selects_before_and_after_opening_a_notebook() {
    for prefix in [' ', ','] {
        let mut app = App::new(Library::demo(), None);
        app.leader = prefix;
        for _ in 0..2 {
            let count = app.library.notebooks.len();
            leader(&mut app, 'n');
            assert!(matches!(app.modal, Some(Modal::Notebook(_))));
            for c in "Created via leader".chars() {
                press(&mut app, KeyCode::Char(c));
            }
            press(&mut app, KeyCode::Enter);
            assert_eq!(app.library.notebooks.len(), count + 1);
            assert_eq!(app.library.book().name, "Created via leader");
            leader(&mut app, 'f');
            assert!(matches!(app.modal, Some(Modal::SelectNotebook(_))));
            press(&mut app, KeyCode::Esc);
        }
    }
}

#[test]
fn home_shortcuts_leave_notebooks_and_allow_reopening_them() {
    let mut app = selected_app();
    for use_super in [true, false] {
        app.action(Action::Notebook(0));
        app.open(Tab::Notes);
        app.input = "A question to keep".into();
        let notes = app.library.book().notes.clone();
        let cards = app.library.book().cards.len();
        if use_super {
            app.editing = true;
            app.modal = Some(Modal::Search("query".into()));
            app.key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::SUPER));
        } else {
            leader(&mut app, 'h');
        }
        assert!(!app.notebook_selected);
        assert!(!app.editing);
        assert!(app.modal.is_none());
        assert_eq!(app.library.book().notes, notes);
        assert_eq!(app.library.book().cards.len(), cards);
        assert_eq!(app.input, "A question to keep");
        leader(&mut app, 'f');
        press(&mut app, KeyCode::Enter);
        assert!(app.notebook_selected);
        assert_eq!(app.library.active, 0);
    }
}

#[test]
fn super_n_creates_notebooks_from_startup_typing_and_card_editor() {
    let mut app = App::new(Library::demo(), None);
    for editing_card in [false, true] {
        if editing_card {
            app.action(Action::Notebook(0));
            app.editing = true;
            app.input = "Keep my question".into();
            app.edit_card();
        }
        app.key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::SUPER));
        assert!(matches!(app.modal, Some(Modal::Notebook(_))));
        assert!(!app.editing);
        for c in "New notebook".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        let count = app.library.notebooks.len();
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.library.notebooks.len(), count + 1);
        assert_eq!(app.library.book().name, "New notebook");
        assert!(app.modal.is_none());
    }
    assert_eq!(app.input, "Keep my question");
}

#[test]
fn study_actions_on_startup_open_picker_without_mutating_a_notebook() {
    let mut app = App::new(Library::demo(), None);
    let count = app.library.book().cards.len();
    leader(&mut app, 'm');
    assert!(matches!(app.modal, Some(Modal::SelectNotebook(_))));
    assert_eq!(app.library.book().cards.len(), count);
    assert!(!app.editing);
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('i'));
    assert!(matches!(app.modal, Some(Modal::SelectNotebook(_))));
    assert!(!app.notebook_selected);
    assert!(!app.editing);
}

#[test]
fn colon_commands_work_on_startup_and_require_confirmation() {
    let mut app = App::new(Library::demo(), None);
    press(&mut app, KeyCode::Char(':'));
    press(&mut app, KeyCode::Char('q'));
    assert!(!app.quit);
    press(&mut app, KeyCode::Esc);
    assert!(!app.quit);
    assert!(app.modal.is_none());
    ex(&mut app, "notebook Biology");
    assert!(app.notebook_selected);
    assert_eq!(app.library.active, 1);
    ex(&mut app, "new Algorithms and data structures");
    assert_eq!(app.library.book().name, "Algorithms and data structures");
    ex(&mut app, "q");
    assert!(app.quit);
}
