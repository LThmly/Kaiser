use super::*;

#[test]
fn import_browser_searches_cycles_completes_and_imports_selected_file() {
    let root = std::env::temp_dir().join(format!("kaiser-import-browser-{}", std::process::id()));
    let folder = root.join("Downloads");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("Lecture One.txt"), "First lecture").unwrap();
    std::fs::write(folder.join("Lecture Two.txt"), "Second lecture").unwrap();
    let mut app = selected_app();
    app.library.book_mut().directory = root.clone();
    leader(&mut app, 'a');
    app.paste("down");
    press(&mut app, KeyCode::Enter);
    assert!(
        matches!(&app.modal, Some(Modal::Import(query)) if query == &format!("{}{sep}", folder.display(), sep = std::path::MAIN_SEPARATOR))
    );
    app.paste("LECTURE");
    assert_eq!(
        app.import_matches(match app.modal.as_ref().unwrap() {
            Modal::Import(s) => s,
            _ => unreachable!(),
        })
        .unwrap()
        .len(),
        2
    );
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.modal_index, 1);
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.modal_index, 0);
    press(&mut app, KeyCode::BackTab);
    assert_eq!(app.modal_index, 1);
    press(&mut app, KeyCode::Right);
    assert!(matches!(&app.modal, Some(Modal::Import(query)) if query.ends_with("Lecture Two.txt")));
    assert!(app.pending_import.is_none());
    press(&mut app, KeyCode::Enter);
    assert!(app.modal.is_none());
    for _ in 0..500 {
        app.poll_import();
        if app.pending_import.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(app.pending_import.is_none());
    assert_eq!(
        app.library.book().sources.last().unwrap().name,
        "Lecture Two.txt"
    );
    leader(&mut app, 'a');
    app.paste("no-such-file");
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.modal, Some(Modal::Import(_))));
    assert!(app.pending_import.is_none());
    press(&mut app, KeyCode::Esc);
    assert!(app.modal.is_none());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn document_import_finishes_in_the_original_notebook_and_failures_do_not_mutate_data() {
    let mut app = selected_app();
    let (tx, rx) = mpsc::channel();
    app.pending_import = Some(PendingImport {
        receiver: rx,
        notebook: 0,
    });
    app.action(Action::Notebook(1));
    let count = app.library.notebooks[0].sources.len();
    tx.send(Ok(crate::import::ExtractedSource {
        name: "Lecture.pptx".into(),
        content: "## Slide 1\n\nReplication copies data.".into(),
    }))
    .unwrap();
    app.poll_import();
    assert!(app.pending_import.is_none());
    assert_eq!(app.library.active, 1);
    assert!(app.library.book().sources.is_empty());
    assert_eq!(app.library.notebooks[0].sources.len(), count + 1);
    assert!(!app.library.notebooks[0].passages("replication").is_empty());
    let (tx, rx) = mpsc::channel();
    app.pending_import = Some(PendingImport {
        receiver: rx,
        notebook: 1,
    });
    app.dirty = false;
    tx.send(Err("Malformed PDF".into())).unwrap();
    app.poll_import();
    assert!(app.library.book().sources.is_empty());
    assert!(!app.dirty);
    assert_eq!(app.status, "Import failed: Malformed PDF");
}

#[test]
fn notebooks_remember_directories_and_import_relative_to_them() {
    let mut app = selected_app();
    let original_cwd = std::env::current_dir().unwrap();
    let root = std::env::temp_dir().join(format!("kaiser-notebook-dirs-{}", std::process::id()));
    let course = root.join("course notes");
    std::fs::create_dir_all(&course).unwrap();
    std::fs::write(
        course.join("lecture.txt"),
        "Replication copies data across nodes.",
    )
    .unwrap();
    ex(&mut app, &format!("cd {}", course.display()));
    let directory = course.canonicalize().unwrap();
    assert_eq!(app.library.book().directory, directory);
    assert_eq!(std::env::current_dir().unwrap(), original_cwd);
    ex(&mut app, "pwd");
    assert!(app.status.ends_with(&directory.display().to_string()));
    ex(&mut app, "cd lecture.txt");
    assert!(app.status.starts_with("Directory unchanged"));
    assert_eq!(app.library.book().directory, directory);
    ex(&mut app, "new Course exercises");
    assert_eq!(app.library.book().directory, directory);
    app.action(Action::Notebook(0));
    let count = app.library.book().sources.len();
    ex(&mut app, "import lecture.txt");
    app.action(Action::Notebook(1));
    for _ in 0..500 {
        app.poll_import();
        if app.pending_import.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(app.pending_import.is_none());
    assert_eq!(app.library.notebooks[0].sources.len(), count + 1);
    assert_eq!(
        app.library.notebooks[0].sources.last().unwrap().name,
        "lecture.txt"
    );
    assert!(app.library.book().sources.is_empty());
    app.action(Action::Notebook(0));
    assert_eq!(app.library.book().directory, directory);
    std::fs::remove_dir_all(root).unwrap();
}
