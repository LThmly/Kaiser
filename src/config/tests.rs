use super::*;

#[test]
fn documented_defaults_match_omitted_settings_and_runtime_defaults() {
    let documented: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
    let omitted: Config = toml::from_str("").unwrap();
    documented.validate().unwrap();
    for config in [omitted, Config::default()] {
        assert_eq!(
            serde_json::to_value(config).unwrap(),
            serde_json::to_value(&documented).unwrap()
        );
    }
}

#[test]
fn usage_errors_name_the_invalid_setting() {
    let config: Config = toml::from_str("[model.usage]\nsource_bytes = 262145").unwrap();
    let error = config.validate().unwrap_err().to_string();
    assert!(error.contains("model.usage.source_bytes"));
    assert!(error.contains("max_input_bytes"));
}

#[test]
fn built_in_theme_registry_has_unique_canonical_names() {
    let mut names = std::collections::HashSet::new();
    for (name, raw) in BUILTIN_THEMES {
        assert!(names.insert(*name), "duplicate {name}");
        assert_eq!(canonical_theme_name(name), *name);
        let theme = parse_theme(raw).unwrap();
        assert_eq!(
            theme,
            load_theme(Path::new("/unused/config.toml"), name).unwrap()
        );
    }
    assert!(names.contains(Config::default().theme.as_str()));
}

fn root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("kaiser-config-{}-{label}", std::process::id()))
}

#[test]
fn shipped_theme_matches_helix_and_embedded_default() {
    let config: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
    assert_eq!(config.theme, "kaiser-tallgeese");
    assert_eq!(config.keys.character().unwrap(), ' ');
    assert_eq!(parse_theme(TALLGEESE_THEME).unwrap(), Theme::tallgeese());
    assert_eq!(Theme::tallgeese().accent, Color::Rgb(247, 71, 94));
}
#[test]
fn leader_defaults_and_customization_are_validated() {
    let config: Config = toml::from_str("[keys]\nleader = ','").unwrap();
    assert_eq!(config.keys.character().unwrap(), ',');
    let config: Config = toml::from_str("").unwrap();
    assert_eq!(config.keys.character().unwrap(), ' ');
    assert!(
        KeysConfig {
            leader: "ctrl-space".into()
        }
        .character()
        .is_err()
    );
    assert!(
        KeysConfig {
            leader: "\n".into()
        }
        .character()
        .is_err()
    );
}
#[test]
fn custom_theme_overrides_a_base_and_model_settings_load() {
    let root = root("custom");
    let path = root.join("config.toml");
    initialize(&path).unwrap();
    fs::write(&path, "theme = 'personal'\n[ui]\nsidebar = false\n[model]\napi_url = 'http://localhost:1234/v1'\nmodel = 'test'\n").unwrap();
    fs::write(
        root.join("themes/personal.toml"),
        "base = 'kaiser'\naccent = '#123aBC'\n",
    )
    .unwrap();
    let (config, theme) = load(&path, true).unwrap();
    assert!(!config.ui.sidebar);
    assert_eq!(config.model.model.as_deref(), Some("test"));
    assert_eq!(theme.accent, Color::Rgb(18, 58, 188));
    assert_eq!(
        theme.background,
        Theme::builtin("kaiser").unwrap().background
    );
    initialize(&path).unwrap();
    assert_eq!(load(&path, true).unwrap().0.theme, "personal");
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn rejects_invalid_color_unknown_fields_and_missing_explicit_config() {
    assert!(parse_theme("accent = '#12💥'").is_err());
    assert!(parse_theme("accent = 'red'").is_err());
    assert!(parse_theme("accnet = '#123456'").is_err());
    let path = root("missing").join("config.toml");
    assert!(load(&path, true).is_err());
    assert_eq!(load(&path, false).unwrap().1, Theme::tallgeese());
}
#[test]
fn bagels_theme_is_installed_discoverable_and_matches_builtin() {
    let root = root("bagels-theme");
    let path = root.join("config.toml");
    initialize(&path).unwrap();
    assert_eq!(
        parse_theme(BAGELS_THEME).unwrap(),
        Theme::builtin("bagels").unwrap()
    );
    assert!(
        theme_names(&path)
            .unwrap()
            .contains(&"kaiser-bagels".into())
    );
    assert_eq!(
        load_theme(&path, "bagels").unwrap(),
        Theme::builtin("bagels").unwrap()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn builtin_theme_can_be_opened_without_overwriting_existing_changes() {
    let root = root("theme-edit");
    let path = root.join("config.toml");
    let file = editable_theme(&path, "kaiser").unwrap();
    assert_eq!(
        load_theme(&path, "kaiser").unwrap(),
        Theme::builtin("kaiser").unwrap()
    );
    fs::write(&file, "accent = '#123456'").unwrap();
    editable_theme(&path, "kaiser").unwrap();
    assert_eq!(fs::read_to_string(file).unwrap(), "accent = '#123456'");
    assert!(
        theme_names(&path)
            .unwrap()
            .contains(&"kaiser-lavender".into())
    );
    assert!(theme_path(&path, "../escape").is_err());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn all_presets_load_install_and_support_custom_inheritance() {
    let root = root("all-themes");
    let path = root.join("config.toml");
    let names = builtin_theme_names();
    assert_eq!(names.len(), 14);
    // Built-ins work before any theme files have been installed.
    for name in &names {
        let theme = load_theme(&path, name).unwrap();
        assert_eq!(parse_theme(&format!("base = '{name}'")).unwrap(), theme);
    }
    initialize(&path).unwrap();
    assert_eq!(theme_names(&path).unwrap(), names);
    for (name, raw) in BUILTIN_THEMES {
        let theme = Theme::builtin(name).unwrap();
        assert_eq!(parse_theme(raw).unwrap(), theme);
        assert_eq!(load_theme(&path, name).unwrap(), theme);
        assert!(editable_theme(&path, name).unwrap().is_file());
    }
    let ember = root.join("themes/kaiser-ember.toml");
    fs::write(&ember, "base = 'ember'\naccent = '#123456'\n").unwrap();
    initialize(&path).unwrap();
    assert_eq!(
        load_theme(&path, "ember").unwrap().accent,
        Color::Rgb(18, 52, 86)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_theme_names_resolve_to_one_canonical_palette_and_path() {
    let root = root("theme-aliases");
    let path = root.join("config.toml");
    for (alias, canonical) in THEME_ALIASES {
        assert_eq!(
            load_theme(&path, alias).unwrap(),
            load_theme(&path, canonical).unwrap()
        );
        assert_eq!(
            theme_path(&path, alias).unwrap(),
            theme_path(&path, canonical).unwrap()
        );
        assert!(is_builtin_theme(alias));
    }
    assert!(
        builtin_theme_names()
            .iter()
            .all(|name| name.starts_with("kaiser-"))
    );
    assert_eq!(
        load_theme(&path, "kaiser-main").unwrap().home_border,
        Some(Color::Rgb(73, 56, 60))
    );
}

#[test]
fn legacy_customizations_migrate_without_overwriting_or_duplicate_suggestions() {
    let root = root("theme-migration");
    let path = root.join("config.toml");
    fs::create_dir_all(root.join("themes")).unwrap();
    let legacy = root.join("themes/kaiser-home.toml");
    let raw = "base = 'tallgeese'\nborder = '#123456'\naccent = '#ABCDEF'\n";
    fs::write(&legacy, raw).unwrap();
    let before = load_theme(&path, "kaiser-main").unwrap();
    assert_eq!(before.home_border, Some(Color::Rgb(18, 52, 86)));
    initialize(&path).unwrap();
    let canonical = root.join("themes/kaiser-main.toml");
    assert_eq!(fs::read_to_string(&canonical).unwrap(), raw);
    assert_eq!(fs::read_to_string(&legacy).unwrap(), raw);
    assert_eq!(load_theme(&path, "kaiser-home").unwrap(), before);
    assert_eq!(editable_theme(&path, "kaiser-home").unwrap(), canonical);
    assert_eq!(theme_names(&path).unwrap(), builtin_theme_names());
    fs::write(
        &canonical,
        "base = 'kaiser-main'\nhome_border = '#654321'\n",
    )
    .unwrap();
    initialize(&path).unwrap();
    assert_eq!(
        load_theme(&path, "kaiser-main").unwrap().home_border,
        Some(Color::Rgb(101, 67, 33))
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn editor_argv_preserves_literal_arguments() {
    let editor = EditorConfig {
        command: vec!["/path with spaces/editor".into(), "--wait".into()],
    };
    assert_eq!(editor.argv().unwrap(), editor.command);
    assert!(
        EditorConfig {
            command: vec![String::new()]
        }
        .argv()
        .is_err()
    );
}
