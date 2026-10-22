use super::*;

#[test]
fn summary_review_preserves_input_and_rechecks_changed_sources() {
    let mut app = selected_app();
    app.library.book_mut().sources = vec![crate::model::Source {
        id: 99,
        name: "chapter-1.pdf".into(),
        content: format!(
            "Chapter 1\n\n{}",
            "Meaningful evidence about learning.\n\n".repeat(1300)
        ),
        selected: true,
    }];
    app.provider = Some(
        Provider::chatgpt(
            crate::auth::Store::new(std::env::temp_dir().join("kaiser-unused-summary-auth")),
            "synthetic".into(),
            Some("chosen".into()),
        )
        .with_usage(app.ai.clone()),
    );
    app.input = "Summarize chapter 1".into();
    let before = app.library.book().messages.len();
    app.send();
    assert!(matches!(app.modal, Some(Modal::SummaryPlan { requests, .. }) if requests > 1));
    assert_eq!(app.modal_index, 0);
    assert!(app.pending.is_none());
    assert_eq!(app.input, "Summarize chapter 1");
    assert_eq!(app.library.book().messages.len(), before);
    app.action(Action::ConfirmSummary(false));
    assert!(app.modal.is_none());
    app.send();
    app.library.book_mut().sources[0]
        .content
        .push_str("New evidence.");
    app.action(Action::ConfirmSummary(true));
    assert!(matches!(app.modal, Some(Modal::SummaryPlan { .. })));
    assert!(app.pending.is_none());
    assert_eq!(app.library.book().messages.len(), before);
}
#[test]
fn clear_chat_only_removes_current_history_and_discards_its_pending_reply() {
    let mut app = selected_app();
    let original = app.library.clone();
    app.input = "Unsent question".into();
    app.scroll = Some(10);
    let (sender, receiver) = mpsc::channel();
    let (progress, progress_receiver) = mpsc::channel();
    app.pending = Some(Pending {
        kind: JobKind::Chat,
        receiver,
        notebook: 0,
        citations: vec![],
    });
    app.progress_receiver = Some(progress_receiver);
    app.generation_progress = "Summary in progress".into();
    app.execute_ex("clear");
    assert!(app.library.book().messages.is_empty());
    assert!(app.input.is_empty());
    assert!(app.pending.is_none());
    assert!(app.generation_progress.is_empty());
    assert!(sender.send(Ok("Late reply".into())).is_err());
    assert!(progress.send("Next step".into()).is_err());
    app.poll_answer();
    assert!(app.library.book().messages.is_empty());
    assert!(app.dirty);
    assert_eq!(app.scroll, None);
    let mut expected = original;
    expected.book_mut().messages.clear();
    assert_eq!(
        serde_json::to_value(&app.library).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    assert!(
        ex_matches("clear")
            .iter()
            .any(|&i| EX_COMMANDS[i].0 == "clear")
    );

    let (sender, receiver) = mpsc::channel();
    app.pending = Some(Pending {
        kind: JobKind::Chat,
        receiver,
        notebook: 1,
        citations: vec![],
    });
    app.execute_ex("clear");
    assert!(app.pending.is_some());
    sender.send(Ok("Other notebook reply".into())).unwrap();
    app.poll_answer();
    assert_eq!(
        app.library.notebooks[1].messages.last().unwrap().text,
        "Other notebook reply"
    );

    app.go_home();
    app.execute_ex("clear");
    assert!(matches!(app.modal, Some(Modal::SelectNotebook(_))));
}
#[test]
fn chat_composer_preserves_multiline_paste_and_shift_enter() {
    let mut app = selected_app();
    app.action(Action::ComposeChat);
    app.paste("First paragraph\nSecond paragraph");
    app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
    app.paste("Last paragraph");
    assert_eq!(
        app.input,
        "First paragraph\nSecond paragraph\nLast paragraph"
    );
    assert!(app.pending.is_none());
    let count = app.library.book().messages.len();
    app.action(Action::SendChat);
    assert_eq!(
        app.library.book().messages[count].text,
        "First paragraph\nSecond paragraph\nLast paragraph"
    );
    assert!(app.input.is_empty());
    app.action(Action::ComposeChat);
    app.action(Action::Source(0));
    assert!(!app.editing);
}
#[test]
fn ai_usage_is_local_and_oversized_questions_preserve_the_input_and_conversation() {
    let mut app = selected_app();
    app.execute_ex("ai-usage");
    assert!(
        matches!(&app.modal, Some(Modal::AiUsage(text)) if text.contains("Reported input tokens"))
    );
    press(&mut app, KeyCode::PageDown);
    press(&mut app, KeyCode::Home);
    assert_eq!(app.modal_index, 0);
    press(&mut app, KeyCode::Esc);
    app.provider = Some(
        Provider::chatgpt(
            crate::auth::Store::new(std::env::temp_dir().join("kaiser-unused-budget-auth")),
            "synthetic".into(),
            Some("chosen".into()),
        )
        .with_usage(app.ai.clone()),
    );
    app.input = "Long question ".repeat(1000);
    let question = app.input.clone();
    let previous = app.library.book().messages.len();
    app.send();
    assert_eq!(app.input, question);
    assert_eq!(app.library.book().messages.len(), previous);
    assert!(app.pending.is_none());
    assert!(app.status.contains("input budget"));
}
#[test]
fn follow_ups_reuse_only_selected_evidence_and_unrelated_queries_do_not() {
    let mut app = selected_app();
    app.input = "Why?".into();
    app.send();
    assert!(
        !app.library
            .book()
            .messages
            .last()
            .unwrap()
            .citations
            .is_empty()
    );
    app.library.book_mut().sources[0].selected = false;
    app.input = "Why?".into();
    app.send();
    assert!(
        app.library
            .book()
            .messages
            .last()
            .unwrap()
            .citations
            .is_empty()
    );
    app.input = "What are mitochondria?".into();
    app.send();
    assert!(
        app.library
            .book()
            .messages
            .last()
            .unwrap()
            .citations
            .is_empty()
    );
}
