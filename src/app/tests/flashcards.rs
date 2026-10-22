use super::*;

#[test]
fn generated_cards_are_cited_drafts_in_the_origin_notebook_and_validate_atomically() {
    let mut app = selected_app();
    let original_messages = serde_json::to_value(&app.library.book().messages).unwrap();
    let original_count = app.library.book().cards.len();
    let citation = Citation {
        source_id: 42,
        source_name: "source.pdf".into(),
        line: 12,
        passage: "A fact".into(),
    };
    let complete = |app: &mut App, text: &str| {
        let (sender, receiver) = mpsc::channel();
        app.pending = Some(Pending {
            kind: JobKind::Cards(3),
            receiver,
            notebook: 0,
            citations: vec![citation.clone()],
        });
        sender.send(Ok(text.into())).unwrap();
        app.poll_answer();
    };
    app.action(Action::Notebook(1));
    let response = r#"[{"front":"A new concept?","back":"An explanation.","source":1},{"front":"a  new concept?","back":"Duplicate","source":1}]"#;
    complete(&mut app, response);
    assert_eq!(app.library.active, 1);
    assert_eq!(app.library.notebooks[0].cards.len(), original_count + 1);
    assert_eq!(
        serde_json::to_value(&app.library.notebooks[0].messages).unwrap(),
        original_messages
    );
    let card = app.library.notebooks[0].cards.last().unwrap();
    assert!(card.draft && card.history.is_empty());
    assert_eq!(card.citation.as_ref().unwrap().source_id, 42);
    complete(&mut app, response);
    assert_eq!(app.library.notebooks[0].cards.len(), original_count + 1);
    complete(
        &mut app,
        r#"[{"front":"Another?","back":"Answer","source":1},{"front":"Broken","back":"Answer","source":9}]"#,
    );
    assert_eq!(app.library.notebooks[0].cards.len(), original_count + 1);
    assert!(app.status.contains("invalid source citation"));
    app.action(Action::Notebook(0));
    app.execute_ex("drafts");
    let index = app
        .cards()
        .iter()
        .position(|card| card.front == "A new concept?")
        .unwrap();
    app.select_card(index);
    app.edit_card();
    ctrl(&mut app, 'a');
    assert!(!app.library.book().cards.last().unwrap().draft);
}
#[test]
fn card_generation_requires_a_model_and_reviews_large_scopes_before_requests() {
    let mut app = selected_app();
    app.execute_ex("generate-cards");
    assert!(app.pending.is_none());
    assert!(app.status.contains("Connect a model"));
    app.library.book_mut().sources = vec![crate::model::Source {
        id: 98,
        name: "chapter-1.pdf".into(),
        selected: true,
        content: format!(
            "Chapter 1\n\n{}",
            "Important evidence and examples.\n\n".repeat(1400)
        ),
    }];
    app.provider = Some(
        Provider::chatgpt(
            crate::auth::Store::new(std::env::temp_dir().join("kaiser-unused-card-auth")),
            "synthetic".into(),
            Some("chosen".into()),
        )
        .with_usage(app.ai.clone()),
    );
    app.input = "Unsent chat question".into();
    let original = serde_json::to_value(&app.library).unwrap();
    app.execute_ex("generate-cards 5 chapter 1");
    assert!(
        matches!(&app.modal, Some(Modal::SummaryPlan { cards: Some(request), requests, .. }) if request.count == 5 && *requests > 1)
    );
    assert_eq!(app.modal_index, 0);
    assert!(app.pending.is_none());
    app.library.book_mut().sources[0]
        .content
        .push_str("Additional evidence.");
    app.action(Action::ConfirmSummary(true));
    assert!(matches!(
        &app.modal,
        Some(Modal::SummaryPlan { cards: Some(_), .. })
    ));
    assert!(app.pending.is_none());
    app.action(Action::ConfirmSummary(false));
    app.library.book_mut().sources[0].content.truncate(
        original["notebooks"][0]["sources"][0]["content"]
            .as_str()
            .unwrap()
            .len(),
    );
    assert_eq!(serde_json::to_value(&app.library).unwrap(), original);
    assert_eq!(app.input, "Unsent chat question");
    app.execute_ex("generate-cards 0");
    assert!(app.status.contains("1–20"));
    app.go_home();
    app.execute_ex("generate-cards");
    assert!(matches!(app.modal, Some(Modal::SelectNotebook(_))));
}
#[test]
fn question_to_draft_to_saved_card_to_browser() {
    let mut app = selected_app();
    press(&mut app, KeyCode::Char('i'));
    app.paste("What is replication lag?");
    press(&mut app, KeyCode::Enter);
    assert!(
        !app.library
            .book()
            .messages
            .last()
            .unwrap()
            .citations
            .is_empty()
    );
    leader(&mut app, 'm');
    ctrl(&mut app, 's');
    let id = app.library.book().cards.last().unwrap().id;
    assert!(app.library.book().cards.last().unwrap().draft);
    leader(&mut app, 'd');
    app.card = app.cards().len() - 1;
    press(&mut app, KeyCode::Char('e'));
    ctrl(&mut app, 'a');
    assert!(
        !app.library
            .book()
            .cards
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .draft
    );
    app.drafts_only = false;
    app.open(Tab::Cards);
    app.select_card(
        app.library
            .book()
            .cards
            .iter()
            .position(|c| c.id == id)
            .unwrap(),
    );
    press(&mut app, KeyCode::Enter);
    assert!(app.revealed);
    assert_eq!(app.current_card().unwrap().id, id);
    assert!(app.current_card().unwrap().history.is_empty());
}

#[test]
fn flashcards_are_browsed_one_at_a_time_without_sessions_or_ratings() {
    let mut app = selected_app();
    app.open(Tab::Cards);
    assert_eq!(app.current_card().unwrap().id, 4);
    press(&mut app, KeyCode::Enter);
    assert!(app.revealed);
    press(&mut app, KeyCode::Right);
    assert_eq!(app.current_card().unwrap().id, 5);
    assert!(!app.revealed);
    press(&mut app, KeyCode::Right);
    assert_eq!(app.current_card().unwrap().id, 6); // Drafts are browsable too.
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('3'));
    assert!(app.revealed);
    assert!(app.current_card().unwrap().history.is_empty());
    press(&mut app, KeyCode::Left);
    assert_eq!(app.current_card().unwrap().id, 5);
    assert!(!app.revealed);
    leader(&mut app, 'e');
    press(&mut app, KeyCode::Down);
    assert_eq!(app.current_card().unwrap().id, 6);
    press(&mut app, KeyCode::Enter);
    assert!(app.focus == Focus::Workspace);
    press(&mut app, KeyCode::Enter);
    assert!(app.revealed);
    press(&mut app, KeyCode::Char('e'));
    assert!(matches!(app.modal, Some(Modal::Card { id: Some(6), .. })));
}

#[test]
fn revealing_saved_cards_tracks_daily_progress_without_counting_repeat_flips_or_drafts() {
    let mut app = selected_app();
    app.open(Tab::Cards);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.library.review_activity.len(), 1);
    assert_eq!(app.library.review_activity[0].cards, vec![4]);
    app.dirty = false;
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    assert!(!app.dirty);
    assert_eq!(app.library.review_activity[0].cards, vec![4]);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.library.review_activity[0].cards, vec![4, 5]);
    app.dirty = false;
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Enter);
    assert!(!app.dirty);
    assert_eq!(app.library.review_activity[0].cards, vec![4, 5]);
}

#[test]
fn flashcards_panel_opens_terms_and_edits_them_only_on_e() {
    let mut app = selected_app();
    leader(&mut app, 'e');
    press(&mut app, KeyCode::Down);
    assert_eq!(app.flashcard, 1);
    assert_eq!(app.current_card().unwrap().id, 5);
    assert!(!app.revealed);
    press(&mut app, KeyCode::End);
    assert_eq!(app.flashcard, 2);
    press(&mut app, KeyCode::Enter);
    assert!(app.modal.is_none());
    assert_eq!(app.current_card().unwrap().id, 6);
    press(&mut app, KeyCode::Char('e'));
    assert!(matches!(app.modal, Some(Modal::Card { id: Some(6), .. })));
    press(&mut app, KeyCode::Esc);
    app.action(Action::Notebook(1));
    leader(&mut app, 'e');
    press(&mut app, KeyCode::Down);
    assert_eq!(app.flashcard, 0);
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.modal, Some(Modal::Card { id: None, .. })));
}
