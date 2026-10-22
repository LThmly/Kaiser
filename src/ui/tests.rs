use super::*;
use crate::model::Library;
use ratatui::{Terminal, backend::TestBackend};
#[test]
fn chat_is_a_full_height_conversation_with_markdown_and_clickable_citations() {
    for (width, height) in [(58, 18), (120, 40)] {
        let mut app = App::new(Library::fixture(), None);
        app.action(Action::Notebook(0));
        let citation = app.library.book().messages.last().unwrap().citations[0].clone();
        app.library.book_mut().messages = vec![
            crate::model::Message { role: "user".into(), text: "Explain replication".into(), citations: vec![] },
            crate::model::Message { role: "assistant".into(), text: "**Replication** copies data.\n\n- Copies support availability.\n- Lag affects reads.".into(), citations: vec![citation] },
        ];
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(text.contains("Send"));
        assert!(!text.contains("**Replication**"));
        assert!(!text.contains("Enter sends"));
        assert!(
            !app.hits
                .iter()
                .any(|(_, action)| matches!(action, Action::SelectFlashcard(_)))
        );
        assert!(
            app.hits
                .iter()
                .any(|(_, action)| matches!(action, Action::ComposeChat))
        );
        let citation_action = app
            .hits
            .iter()
            .find_map(|(_, action)| matches!(action, Action::ReadSource(_)).then_some(*action))
            .unwrap();
        app.action(citation_action);
        assert!(matches!(app.modal, Some(Modal::Reader)));
        assert!(app.tab == Tab::Chat);
    }
    assert_eq!(composer_lines("ab 🦀\n", 5), vec!["ab 🦀", ""]);
    assert_eq!(composer_lines("12345", 5), vec!["12345", ""]);
    assert_eq!(composer_lines("a  ", 5), vec!["a  "]);
}
#[test]
fn notebook_deletion_confirmation_and_empty_home_render_at_supported_sizes() {
    for (width, height) in [(58, 18), (120, 40)] {
        let mut app = App::new(Library::fixture(), None);
        app.modal = Some(Modal::DeleteNotebook {
            notebook: 0,
            query: String::new(),
            selected: 0,
        });
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(text.contains("Delete notebook?"));
        assert!(
            app.hits
                .iter()
                .any(|(_, action)| matches!(action, Action::ConfirmNotebookDeletion(true)))
        );
        app.library.notebooks.clear();
        app.library.active = 0;
        app.modal = None;
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(text.contains("Notebooks · 0"));
        assert!(text.contains("Make new notebooks"));
    }
}
#[test]
fn home_borders_use_theme_accents_while_main_preserves_its_original_color() {
    let mut app = App::new(Library::fixture(), None);
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    for name in crate::config::builtin_theme_names() {
        app.theme =
            crate::config::load_theme(std::path::Path::new("/unused/config.toml"), &name).unwrap();
        for transparent in [false, true] {
            app.transparent_background = transparent;
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            let expected = if name == "kaiser-main" {
                Color::Rgb(73, 56, 60)
            } else {
                app.theme.accent
            };
            let buffer = terminal.backend().buffer();
            assert_eq!(buffer[(1, 0)].fg, expected, "{name}");
            assert_eq!(buffer[(75, 0)].fg, expected, "{name}");
            assert_eq!(
                buffer[(0, 39)].bg,
                if transparent { Color::Reset } else { STATUS_FG }
            );
        }
    }
}
#[test]
fn notes_render_markdown_and_source_reader_does_not_add_a_workspace_tab() {
    let mut app = App::new(Library::fixture(), None);
    app.action(Action::Notebook(0));
    app.library.book_mut().notes = "# Topic\n\n**Strong** and `code`".into();
    app.open(Tab::Notes);
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(text.contains("MARKDOWN"));
    assert!(text.contains("Strong"));
    assert!(!text.contains("**Strong**"));
    assert_eq!(
        app.hits
            .iter()
            .filter(|(_, action)| matches!(action, Action::Tab(_)))
            .count(),
        3
    );
    app.notes.preview = false;
    terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(text.contains("**Strong**"));
    app.read_source();
    terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(text.contains("Source reader"));
    assert!(text.contains("replication.md"));
    assert!(app.tab == Tab::Notes);
}
#[test]
fn home_lists_current_notebooks_and_opens_the_clicked_notebook() {
    let mut app = App::new(Library::fixture(), None);
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(text.contains("Select a notebook"));
    assert!(!text.contains("understand · practice · remember"));
    assert!(!text.contains("Get started"));
    assert!(!text.contains("Progress"));
    assert!(!text.contains("review streak"));
    assert!(!text.contains("Name"));
    assert!(!text.contains("Count"));
    assert!(!text.contains(": commands"));
    assert!(!text.contains(":theme appearance"));
    for cell in &terminal.backend().buffer().content {
        if matches!(cell.symbol(), "●" | "├" | "└") {
            assert_eq!(cell.fg, app.theme.muted);
        }
    }
    assert!(text.contains("├─ Sources"));
    assert!(text.contains("├─ Flashcards"));
    assert!(text.contains("└─"));
    assert!(!text.contains("+ New notebook"));
    assert!(!text.contains("Browse all"));
    assert!(!text.contains("Mastery"));
    assert!(!text.contains("Unrated"));
    assert!(text.contains("Notebooks · 3"));
    assert!(text.contains("Distributed systems"));
    assert!(text.contains("Biology"));
    assert!(text.contains("Reading room"));
    assert!(!text.contains("Library"));
    let biology = app
        .hits
        .iter()
        .find_map(|(_, action)| matches!(action, Action::Notebook(1)).then_some(*action))
        .unwrap();
    app.action(biology);
    assert!(app.notebook_selected);
    assert_eq!(app.library.active, 1);
    for hidden in [
        "replication.md",
        "Study queue",
        "Card preview",
        "╱",
        "Workspace",
        "Open notebook",
    ] {
        assert!(
            !text.contains(hidden),
            "unexpected startup content: {hidden}"
        );
    }
    app.action(Action::Notebook(0));
    terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(text.contains("Distributed systems"));
    assert!(text.contains("replication.md"));
    assert!(!text.contains("Biology"));
    assert!(!text.contains("Reading room"));
    assert!(!text.contains("Study queue"));
    assert!(
        app.hits
            .iter()
            .any(|(_, action)| matches!(action, Action::PickNotebook))
    );
}
#[test]
fn transparency_covers_all_theme_surfaces_and_can_be_disabled() {
    for name in ["tallgeese", "bagels", "kaiser"] {
        let mut app = App::new(Library::fixture(), None);
        app.theme =
            crate::config::load_theme(std::path::Path::new("/unused/config.toml"), name).unwrap();
        app.transparent_background = true;
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .all(|cell| cell.bg == Color::Reset)
        );
        app.action(Action::Notebook(0));
        for tab in [Tab::Chat, Tab::Cards, Tab::Notes] {
            app.action(Action::Tab(tab));
            for modal in [
                None,
                Some(Modal::Leader),
                Some(Modal::Ex("theme".into())),
                Some(Modal::SelectNotebook(String::new())),
                Some(Modal::Help),
            ] {
                app.modal = modal;
                terminal.draw(|frame| draw(frame, &mut app)).unwrap();
                assert!(
                    terminal
                        .backend()
                        .buffer()
                        .content
                        .iter()
                        .all(|cell| cell.bg == Color::Reset)
                );
            }
        }
        app.transparent_background = false;
        app.modal = None;
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        assert_eq!(terminal.backend().buffer()[(0, 0)].bg, app.theme.background);
        assert_eq!(terminal.backend().buffer()[(0, 39)].bg, STATUS_FG);
        app.transparent_background = true;
        let mut small = Terminal::new(TestBackend::new(20, 8)).unwrap();
        small.draw(|frame| draw(frame, &mut app)).unwrap();
        assert!(
            small
                .backend()
                .buffer()
                .content
                .iter()
                .all(|cell| cell.bg == Color::Reset)
        );
    }
}

#[test]
fn bottom_status_bar_is_neutral_in_every_mode_and_theme() {
    for name in ["tallgeese", "bagels", "kaiser"] {
        let mut app = App::new(Library::fixture(), None);
        app.theme =
            crate::config::load_theme(std::path::Path::new("/unused/config.toml"), name).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
        for modal in [
            None,
            Some(Modal::Leader),
            Some(Modal::Help),
            Some(Modal::Ex("theme".into())),
            Some(Modal::SelectNotebook(String::new())),
        ] {
            app.modal = modal;
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            for x in 0..120 {
                let cell = &terminal.backend().buffer()[(x, 39)];
                assert!(matches!(cell.fg, Color::Rgb(r, g, b) if r == g && g == b));
                assert!(matches!(cell.bg, Color::Rgb(r, g, b) if r == g && g == b));
            }
        }
    }
}

#[test]
fn empty_notebook_views_render_slashes_and_messages() {
    let mut app = App::new(Library::fixture(), None);
    app.action(Action::Notebook(1));
    app.library.book_mut().sources.clear();
    app.library.book_mut().messages.clear();
    app.library.book_mut().cards.clear();
    app.library.book_mut().notes.clear();
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    for (tab, message) in [
        (Tab::Chat, "No conversation yet"),
        (Tab::Cards, "No flashcards yet"),
        (Tab::Notes, "No notes yet"),
    ] {
        app.action(Action::Tab(tab));
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let buffer = terminal.backend().buffer();
        let text: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
        assert!(text.contains(message), "missing {message}");
        assert!(text.contains("╱╱╱╱"));
        assert!(text.contains("No sources yet"));
        // The workspace and context never receive the Sources-only pattern.
        for y in 0..40 {
            for x in 29..120 {
                assert_ne!(buffer[(x, y)].symbol(), "╱");
            }
        }
        // Full-cell diagonal strokes continue through adjacent rows.
        for y in [8, 9, 10] {
            for x in [3, 4, 5] {
                assert_eq!(buffer[(x, y)].symbol(), "╱");
                assert_eq!(buffer[(x, y)].fg, app.theme.border);
            }
        }
    }
}

#[test]
fn flashcards_panel_replaces_context_and_remains_accessible_on_small_screens() {
    let mut app = App::new(Library::fixture(), None);
    app.action(Action::Notebook(0));
    for (width, height) in [(120, 40), (58, 18)] {
        app.focus = Focus::Flashcards;
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("Flashcards · 3"));
        assert!(text.contains("read-your-writes"));
        assert!(!text.contains("Context"));
        assert!(!text.contains("Passages"));
        assert!(!text.contains("Card preview"));
        assert!(
            app.hits
                .iter()
                .any(|(_, action)| matches!(action, Action::SelectFlashcard(2)))
        );
    }
}

#[test]
fn directory_label_does_not_recolor_the_workspace_border() {
    let mut app = App::new(Library::fixture(), None);
    app.action(Action::Notebook(0));
    app.library.book_mut().directory = std::path::PathBuf::from("/course");
    for width in [58, 120, 180] {
        let mut terminal = Terminal::new(TestBackend::new(width, 40)).unwrap();
        for focus in [Focus::Workspace, Focus::Sources] {
            app.focus = focus;
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            let buffer = terminal.backend().buffer();
            let area = app
                .hits
                .iter()
                .find_map(|(area, action)| {
                    matches!(action, Action::Focus(Focus::Workspace)).then_some(*area)
                })
                .unwrap();
            let expected = if focus == Focus::Workspace {
                app.theme.accent
            } else {
                app.theme.border
            };
            let mut border_cells = 0;
            for x in area.x..area.right() {
                let cell = &buffer[(x, area.y)];
                if cell.symbol() == "─" {
                    assert_eq!(cell.fg, expected, "width {width}, column {x}");
                    border_cells += 1;
                }
            }
            assert!(border_cells > 5);
            let top: String = (area.x..area.right())
                .map(|x| buffer[(x, area.y)].symbol())
                .collect();
            assert!(top.contains("/course"));
        }
    }
}
#[test]
fn notebook_directory_remains_visible_without_the_navigation_header() {
    let mut app = App::new(Library::fixture(), None);
    app.library.notebooks[0].directory = std::env::temp_dir().join("kaiser-course-one");
    app.library.notebooks[1].directory = std::env::temp_dir().join("kaiser-course-two");
    for width in [58, 120] {
        let mut terminal = Terminal::new(TestBackend::new(width, 40)).unwrap();
        for (index, suffix) in [(0, "kaiser-course-one"), (1, "kaiser-course-two")] {
            app.action(Action::Notebook(index));
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
            let line: String = (0..width)
                .map(|x| terminal.backend().buffer()[(x, 0)].symbol())
                .collect();
            assert!(line.contains(suffix), "{line}");
            assert!(line.contains("Workspace"));
            assert!(!line.contains("Kaiser"));
            assert!(!line.contains(" / "));
        }
    }
}

#[test]
fn configured_colors_reach_the_rendered_terminal() {
    let mut app = App::new(Library::fixture(), None);
    app.theme.accent = Color::Rgb(18, 58, 188);
    app.theme.background = Color::Rgb(1, 2, 3);
    app.theme.panel = Color::Rgb(4, 5, 6);
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    let buffer = terminal.backend().buffer();
    let logo = buffer
        .content
        .iter()
        .find(|cell| cell.symbol() == "K" && cell.modifier.contains(Modifier::BOLD))
        .unwrap();
    assert_eq!(logo.fg, app.theme.foreground);
    assert_eq!(buffer[(0, 0)].bg, app.theme.background);
    assert_eq!(buffer[(31, 5)].bg, app.theme.panel);
    app.modal = Some(Modal::Help);
    terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    assert!(
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .any(|cell| cell.symbol() == "╭"
                && cell.fg == app.theme.accent
                && cell.bg == app.theme.panel)
    );
}
#[test]
fn colon_input_is_rendered_in_bottom_bar_without_covering_startup() {
    let mut app = App::new(Library::fixture(), None);
    app.modal = Some(Modal::Ex("notebook Biology".into()));
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    let buffer = terminal.backend().buffer();
    let last: String = (0..120).map(|x| buffer[(x, 39)].symbol()).collect();
    assert!(last.contains("COMMAND"));
    assert!(last.contains(":notebook Biology"));
    let text: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
    assert!(text.contains("Kaiser"));
    assert!(text.contains("Select a notebook"));
}
#[test]
fn colon_menu_opens_immediately_and_filters_descriptions() {
    let mut app = App::new(Library::fixture(), None);
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    app.modal = Some(Modal::Ex(String::new()));
    terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(text.contains("Commands"));
    assert!(text.contains("Save and quit"));
    assert!(text.contains("Show the keybinding guide"));
    app.modal = Some(Modal::Ex("car".into()));
    terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(text.contains(":cards"));
    assert!(text.contains("Browse flashcards"));
    assert!(!text.contains("Save and quit"));
    app.modal = Some(Modal::Ex("unknown".into()));
    terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(text.contains("No matching commands"));
}
#[test]
fn theme_names_complete_in_the_colon_menu() {
    let mut app = App::new(Library::fixture(), None);
    app.modal = Some(Modal::Ex("theme ".into()));
    app.themes.push("custom".into());
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(text.contains("Themes"));
    assert!(text.contains("Current theme"));
    assert!(text.contains("custom"));
    assert!(text.contains("tallgeese"));
    assert!(text.contains("Built-in theme"));
    app.modal = Some(Modal::Ex("theme kaiser".into()));
    app.ex_completion_prefix = Some("theme ".into());
    app.ex_selected = Some(0);
    terminal.draw(|frame| draw(frame, &mut app)).unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(text.contains("tallgeese"));
    assert!(text.contains("custom"));
}
#[test]
fn citation_groups_keep_file_names_unique_and_show_spanning_pages() {
    let source = crate::model::Source {
        id: 8,
        name: "chapter.pdf".into(),
        content: "## Page 1\nBeginning\n## Page 2\nEnding".into(),
        selected: true,
    };
    let citations = vec![
        crate::model::Citation {
            source_id: 8,
            source_name: source.name.clone(),
            line: 1,
            passage: source.content.clone(),
        },
        crate::model::Citation {
            source_id: 8,
            source_name: source.name.clone(),
            line: 4,
            passage: "Ending".into(),
        },
    ];
    let groups = citation_groups(&citations, &[source]);
    assert_eq!(groups.len(), 1);
    assert!(groups[0].0.contains("[1–2] chapter.pdf"));
    assert!(groups[0].0.contains("Page 1 … Page 2"));
}
#[test]
fn notebook_picker_previews_the_highlighted_match_without_opening_it() {
    let mut app = App::new(Library::fixture(), None);
    app.library.notebooks[0].notes = "# First preview\n\nFirst notebook details".into();
    app.library.notebooks[1].notes = "# Second preview\n\nSecond notebook details".into();
    app.modal = Some(Modal::SelectNotebook(String::new()));
    let active = app.library.active;
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    let text = |terminal: &Terminal<TestBackend>| -> String {
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    };
    terminal
        .draw(|frame| modal_view(frame, &mut app, Modal::SelectNotebook(String::new())))
        .unwrap();
    assert!(text(&terminal).contains("First notebook details"));
    assert!(!text(&terminal).contains("Second notebook details"));
    app.modal_key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Tab,
        crossterm::event::KeyModifiers::NONE,
    ));
    terminal
        .draw(|frame| modal_view(frame, &mut app, Modal::SelectNotebook(String::new())))
        .unwrap();
    assert!(text(&terminal).contains("Second notebook details"));
    assert!(!text(&terminal).contains("First notebook details"));
    assert_eq!(app.library.active, active);
    assert!(
        app.hits
            .iter()
            .any(|(_, action)| matches!(action, Action::Notebook(1)))
    );
    terminal
        .draw(|frame| {
            modal_view(
                frame,
                &mut app,
                Modal::SelectNotebook("no-such-notebook".into()),
            )
        })
        .unwrap();
    assert!(text(&terminal).contains("No matching notebooks"));
    assert!(text(&terminal).contains("No notebook selected"));
    assert!(!text(&terminal).contains("Second notebook details"));
}
#[test]
fn leader_menu_fits_its_commands_without_the_old_extra_space() {
    let mut app = App::new(Library::fixture(), None);
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal
        .draw(|frame| modal_view(frame, &mut app, Modal::Leader))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let text: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
    for (_, label) in LEADER_BINDINGS {
        assert!(text.contains(label), "missing {label}");
    }
    let width = (leader_column_width() * 2 + 9) as u16;
    let height = LEADER_BINDINGS.len().div_ceil(2) as u16 + 4;
    assert!(width < 86 && height < 18);
    assert_eq!(buffer[((120 - width) / 2, (40 - height) / 2)].symbol(), "╭");
}
#[test]
fn renders_all_screens_at_supported_sizes() {
    for (width, height) in [(20, 8), (58, 18), (85, 25), (120, 40), (180, 50)] {
        let mut app = App::new(Library::fixture(), None);
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        app.action(Action::Notebook(0));
        for tab in [Tab::Chat, Tab::Cards, Tab::Notes] {
            app.open(tab);
            terminal.draw(|f| draw(f, &mut app)).unwrap();
        }
        for modal in [
            Modal::SummaryPlan {
                cards: None,
                fingerprint: "test".into(),
                requests: 12,
                label: "chapter 1 · textbook.pdf".into(),
                bytes: 80000,
            },
            Modal::AiUsage("Reported tokens · 🦀\n".repeat(50)),
            Modal::Accounts,
            Modal::ChatgptModels,
            Modal::SignIn(
                "https://auth.openai.com/api/accounts/authorize?client_id=dynamic_agent_client"
                    .into(),
            ),
            Modal::ChatgptWelcome,
            Modal::Help,
            Modal::Leader,
            Modal::Ex(String::new()),
            Modal::Ex("notebook A very long notebook name with Unicode: λ 中文 🦀".into()),
            Modal::SelectNotebook(String::new()),
            Modal::Import(String::new()),
            Modal::Palette(String::new()),
            Modal::Search("replica".into()),
            Modal::Card {
                id: None,
                front: "Why does this happen?".repeat(30),
                back: "An answer with Unicode: λ 中文 🦀".repeat(30),
                field: 1,
                citation: None,
            },
        ] {
            app.modal = Some(modal);
            terminal.draw(|f| draw(f, &mut app)).unwrap();
        }
    }
}
#[test]
fn card_browser_hides_the_definition_until_flipped() {
    let mut app = App::new(Library::fixture(), None);
    app.action(Action::Notebook(0));
    app.open(Tab::Cards);
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal.draw(|f| draw(f, &mut app)).unwrap();
    let buffer = terminal.backend().buffer();
    let text: String = buffer.content.iter().map(|c| c.symbol()).collect();
    assert!(text.contains("Enter  show answer"));
    assert!(!text.contains("The leader may acknowledge"));
    app.revealed = true;
    terminal.draw(|f| draw(f, &mut app)).unwrap();
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(text.contains("The leader may acknowledge"));
}
