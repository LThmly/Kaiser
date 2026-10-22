use super::*;

#[test]
fn invalid_settings_do_not_partially_change_the_running_app() {
    let mut app = selected_app();
    let before = (app.theme, app.theme_name.clone(), app.leader, app.sidebar);
    let mut settings = crate::config::Config {
        theme: "kaiser-ocean".into(),
        ..Default::default()
    };
    settings.keys.leader = ",".into();
    settings.ui.sidebar = false;
    settings.model.usage.max_summary_requests = 0;
    assert!(
        app.apply_settings(settings, crate::config::Theme::default(), None, vec![])
            .is_err()
    );
    assert_eq!(
        (app.theme, app.theme_name.clone(), app.leader, app.sidebar),
        before
    );
}

#[test]
fn config_reload_is_atomic_and_theme_selection_is_temporary() {
    use std::fs;
    let root = std::env::temp_dir().join(format!("kaiser-reload-{}", std::process::id()));
    fs::create_dir_all(root.join("themes")).unwrap();
    let path = root.join("config.toml");
    let raw = "theme = 'custom'\n[keys]\nleader = ','\n[ui]\nsidebar = false\ntransparent_background = true\n[editor]\ncommand = ['helix']\n";
    fs::write(&path, raw).unwrap();
    fs::write(root.join("themes/custom.toml"), "accent = '#123456'\n").unwrap();
    let mut app = selected_app();
    let notes = app.library.book().notes.clone();
    app.config_path = path.clone();
    ex(&mut app, "config-reload");
    assert_eq!(app.leader, ',');
    assert!(!app.sidebar);
    assert!(app.transparent_background);
    assert_eq!(app.theme.accent, ratatui::style::Color::Rgb(18, 52, 86));
    assert_eq!(app.library.book().notes, notes);
    fs::write(&path, "theme = 'missing'\n[keys]\nleader = ';'\n").unwrap();
    ex(&mut app, "config-reload");
    assert!(app.status.starts_with("Config reload failed"));
    assert_eq!(app.leader, ',');
    assert_eq!(app.theme_name, "custom");
    assert!(!app.sidebar);
    assert!(app.transparent_background);
    fs::write(&path, raw).unwrap();
    ex(&mut app, "theme");
    assert!(matches!(&app.modal, Some(Modal::Ex(line)) if line == "theme "));
    assert!(
        app.ex_suggestions("theme ")
            .iter()
            .any(|item| item.label == "custom")
    );
    for c in "kaiser-t".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    press(&mut app, KeyCode::Tab);
    assert!(matches!(&app.modal, Some(Modal::Ex(line)) if line == "theme kaiser-tallgeese"));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.theme_name, "kaiser-tallgeese");
    assert!(app.transparent_background);
    assert_eq!(fs::read_to_string(&path).unwrap(), raw);
    ex(&mut app, "config-reload");
    assert_eq!(app.theme_name, "custom");
    fs::write(root.join("themes/custom.toml"), "accent = 'invalid'\n").unwrap();
    let previous = app.theme;
    ex(&mut app, "theme-reload");
    assert_eq!(app.theme, previous);
    assert!(app.status.starts_with("Theme failed"));
    ex(&mut app, "config-open");
    assert!(matches!(
        app.open_config_requested,
        Some(EditorTarget::Config)
    ));
    ex(&mut app, "theme-open");
    assert!(matches!(
        app.open_config_requested,
        Some(EditorTarget::Theme)
    ));
    fs::write(root.join("themes/custom.toml"), "accent = '#123456'\n").unwrap();
    fs::write(
        &path,
        raw.replace(
            "transparent_background = true",
            "transparent_background = false",
        ),
    )
    .unwrap();
    ex(&mut app, "config-reload");
    assert!(!app.transparent_background);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn theme_tab_completion_cycles_both_directions_and_wraps() {
    let mut app = selected_app();
    app.modal = Some(Modal::Ex("theme ".into()));
    app.themes = vec!["kaiser".into(), "tallgeese".into(), "tau".into()];
    for name in ["kaiser", "tallgeese", "tau", "kaiser"] {
        press(&mut app, KeyCode::Tab);
        assert!(matches!(&app.modal, Some(Modal::Ex(line)) if *line == format!("theme {name}")));
        assert_eq!(app.ex_menu_suggestions("ignored").len(), 3);
    }
    for name in ["tau", "tallgeese", "kaiser", "tau"] {
        press(&mut app, KeyCode::BackTab);
        assert!(matches!(&app.modal, Some(Modal::Ex(line)) if *line == format!("theme {name}")));
    }
    press(&mut app, KeyCode::Esc);
    assert!(app.ex_completion_prefix.is_none());
    app.modal = Some(Modal::Ex("theme ta".into()));
    for name in ["tallgeese", "tau", "tallgeese"] {
        press(&mut app, KeyCode::Tab);
        assert!(matches!(&app.modal, Some(Modal::Ex(line)) if *line == format!("theme {name}")));
    }
    press(&mut app, KeyCode::Backspace);
    assert!(app.ex_completion_prefix.is_none());
    assert!(app.ex_selected.is_none());
}

#[test]
fn theme_completion_previews_live_and_escape_restores_the_original_palette() {
    let mut app = selected_app();
    app.config_path =
        std::env::temp_dir().join(format!("kaiser-preview-{}/config.toml", std::process::id()));
    app.theme_name = "custom-session".into();
    app.theme.accent = ratatui::style::Color::Rgb(1, 2, 3);
    let original = app.theme;
    app.transparent_background = true;
    app.themes = vec!["ocean".into(), "ember".into(), "paper".into()];
    app.modal = Some(Modal::Ex("theme ".into()));
    for (key, name) in [
        (KeyCode::Tab, "ocean"),
        (KeyCode::Tab, "ember"),
        (KeyCode::BackTab, "ocean"),
        (KeyCode::BackTab, "paper"),
    ] {
        press(&mut app, key);
        assert_eq!(app.theme_name, crate::config::canonical_theme_name(name));
        assert_eq!(
            app.theme,
            crate::config::load_theme(&app.config_path, name).unwrap()
        );
        assert!(matches!(app.modal, Some(Modal::Ex(_))));
        assert!(app.transparent_background);
    }
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.theme, original);
    assert_eq!(app.theme_name, "custom-session");
    assert!(app.theme_preview.is_none());

    app.modal = Some(Modal::Ex("theme ocean".into()));
    press(&mut app, KeyCode::Tab);
    let selected = app.theme;
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.theme, selected);
    assert_eq!(app.theme_name, "kaiser-ocean");
    assert!(app.theme_preview.is_none());
    app.modal = Some(Modal::Ex("theme paper".into()));
    press(&mut app, KeyCode::Down);
    assert_eq!(app.theme_name, "kaiser-paper");
    ctrl(&mut app, 'u');
    assert_eq!(app.theme_name, "kaiser-ocean");
    assert_eq!(app.theme, selected);
    app.modal = Some(Modal::Ex("theme paper".into()));
    press(&mut app, KeyCode::Tab);
    app.key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::SUPER));
    assert_eq!(app.theme_name, "kaiser-ocean");
    assert!(app.theme_preview.is_none());
}

#[test]
fn failed_theme_previews_and_commits_leave_the_original_theme_available() {
    let mut app = selected_app();
    let root = std::env::temp_dir().join(format!("kaiser-bad-preview-{}", std::process::id()));
    std::fs::create_dir_all(root.join("themes")).unwrap();
    std::fs::write(root.join("themes/broken.toml"), "accent = 'invalid'").unwrap();
    app.config_path = root.join("config.toml");
    let original = app.theme;
    app.themes = vec!["ocean".into(), "broken".into()];
    app.modal = Some(Modal::Ex("theme ".into()));
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.theme_name, "kaiser-ocean");
    let preview = app.theme;
    press(&mut app, KeyCode::Tab);
    assert_eq!(app.theme, preview);
    assert!(app.status.starts_with("Theme preview failed"));
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.theme, original);
    assert_eq!(app.theme_name, "kaiser-tallgeese");
    assert!(app.status.starts_with("Theme failed"));
    assert!(app.theme_preview.is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn tab_after_bare_theme_cycles_names_and_enter_applies_the_selection() {
    let mut app = selected_app();
    app.config_path = std::env::temp_dir().join(format!(
        "kaiser-completion-{}/config.toml",
        std::process::id()
    ));
    app.modal = Some(Modal::Ex("theme".into()));
    press(&mut app, KeyCode::Tab);
    assert!(matches!(&app.modal, Some(Modal::Ex(line)) if line == "theme kaiser-bagels"));
    press(&mut app, KeyCode::Tab);
    assert!(matches!(&app.modal, Some(Modal::Ex(line)) if line == "theme kaiser-copper"));
    press(&mut app, KeyCode::BackTab);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.theme_name, "kaiser-bagels");
    assert!(app.modal.is_none());
    assert!(app.ex_completion_prefix.is_none());
}
