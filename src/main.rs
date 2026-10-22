mod app;
mod auth;
mod config;
mod demo;
mod files;
mod flashcards;
mod import;
mod logo;
mod markdown;
mod model;
mod notes;
mod progress;
mod provider;
mod retrieval;
#[cfg(test)]
mod test_support;
mod ui;
mod usage;

use app::{Action, App, EditorTarget};
use crossterm::{
    cursor::{Hide, Show},
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyEventKind, KeyboardEnhancementFlags, MouseEventKind, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use model::Library;
use ratatui::{Terminal, backend::TestBackend, layout::Position};
use std::{io, path::PathBuf, time::Duration};

fn main() {
    if let Err(error) = run() {
        eprintln!("kaiser: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut demo = false;
    let mut demo_view = demo::View::default();
    let mut snapshot = false;
    let mut config_path = config::default_path();
    let mut explicit_config = false;
    let mut init_config = false;
    let mut check_config = false;
    let mut size = (120, 40);
    let mut data_dir = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| ".".into()))
                .join(".local/share")
        })
        .join("kaiser");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--demo" => demo = true,
            "--demo-view" => {
                demo_view = args
                    .next()
                    .ok_or("--demo-view needs home, chat, flashcards, or notes")?
                    .parse()?;
                demo = true;
            }
            "--snapshot" => snapshot = true,
            "--config" => {
                config_path = args.next().ok_or("--config needs a file path")?.into();
                explicit_config = true;
            }
            "--init-config" => init_config = true,
            "--check-config" => {
                check_config = true;
                explicit_config = true;
            }
            "--data-dir" => data_dir = args.next().ok_or("--data-dir needs a directory")?.into(),
            "--size" => {
                let value = args.next().ok_or("--size needs WIDTHxHEIGHT")?;
                let (width, height) = value.split_once('x').ok_or("Use --size 120x40")?;
                size = (
                    width.parse::<u16>()?.clamp(20, 300),
                    height.parse::<u16>()?.clamp(8, 150),
                );
            }
            "--help" | "-h" => {
                println!(
                    "Kaiser — a terminal study workspace.\n\nUsage: kaiser [--demo] [--demo-view VIEW] [--data-dir PATH] [--config PATH] [--snapshot] [--size 120x40]\n\n  --demo       Showcase: six notebooks and 84 flashcards; no saved data or AI requests\n  --demo-view  Open home, chat, flashcards, or notes in the disposable demo\n  --data-dir   Store state.json in this directory\n  --config     Read configuration from this TOML file\n  --init-config  Create missing config/theme files, then exit\n  --check-config Validate configuration and theme, then exit\n  --snapshot   Print a plain-text preview and exit\n  --size       Preview dimensions (default 120x40)\n\nDefault config: ~/.config/kaiser/config.toml (or $XDG_CONFIG_HOME/kaiser/config.toml)\nEdit themes/kaiser-tallgeese.toml to customize colors.\n\nOptional model: [model] in config.toml; overrides: KAISER_API_URL (base URL ending /v1), KAISER_MODEL, KAISER_API_KEY\nConfigured models receive your question, recent conversation, and matching selected-source excerpts.\nWithout a model Kaiser performs local source lookup. Press your leader (default Space), then ? for help."
                );
                return Ok(());
            }
            "--version" | "-V" => {
                println!("Kaiser {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            _ => return Err(format!("Unknown option {arg}. Try --help.").into()),
        }
    }
    if init_config {
        config::initialize(&config_path)?;
        // Validate even if files already existed; don't silently accept mistakes.
        config::load(&config_path, true)?;
        println!(
            "Configuration: {}\nTheme: {}/themes/kaiser-tallgeese.toml\nExisting files were preserved.",
            config_path.display(),
            config_path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(std::path::Path::new("."))
                .display()
        );
        return Ok(());
    }
    let config_path = std::path::absolute(config_path)?;
    let (settings, theme) = config::load(&config_path, explicit_config)?;
    // Demo and snapshot sessions never activate configured API credentials or models.
    let provider = if (!demo && !snapshot) || check_config {
        provider::Provider::from_config(&settings.model).map_err(io::Error::other)?
    } else {
        None
    };
    if check_config {
        println!(
            "Configuration valid: {}\nTheme: {}",
            config_path.display(),
            settings.theme
        );
        return Ok(());
    }
    if !demo && !snapshot && !explicit_config {
        config::initialize(&config_path)?;
    }
    let path = data_dir.join("state.json");
    let library = if demo || snapshot {
        demo::library()
    } else {
        Library::load(&path)?
    };
    let mut app = App::new(library, None);
    app.ai
        .configure(
            settings.model.usage.clone(),
            (!demo && !snapshot).then(|| data_dir.join("ai")),
        )
        .map_err(io::Error::other)?;
    let themes = config::theme_names(&config_path)?;
    app.config_path = config_path;
    app.apply_settings(settings, theme, provider, themes)
        .map_err(io::Error::other)?;
    if !demo && !snapshot {
        app.initialize_auth(auth::Store::new(data_dir.join("auth")));
    }
    if demo {
        demo::prepare(&mut app, demo_view);
    }
    if snapshot {
        let mut terminal = Terminal::new(TestBackend::new(size.0, size.1))?;
        terminal.draw(|frame| ui::draw(frame, &mut app))?;
        let buffer = terminal.backend().buffer();
        for y in 0..size.1 {
            let line: String = (0..size.0).map(|x| buffer[(x, y)].symbol()).collect();
            println!("{}", line.trim_end());
        }
        return Ok(());
    }
    // Persist before entering raw mode so permission errors remain readable.
    if !demo {
        app.library.save(&path)?;
    }
    let mut terminal = ratatui::init();
    struct Restore;
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = execute!(
                io::stdout(),
                PopKeyboardEnhancementFlags,
                DisableMouseCapture,
                DisableBracketedPaste
            );
            ratatui::restore();
        }
    }
    let _restore = Restore;
    app.logo = logo::detect();
    execute!(
        io::stdout(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES),
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    while !app.quit {
        app.poll_answer();
        app.poll_import();
        app.poll_account();
        if (app.dirty || app.save_requested) && !demo {
            match app.library.save(&path) {
                Ok(()) => {
                    app.dirty = false;
                    if app.save_requested {
                        app.status = "Library saved.".into();
                    }
                }
                Err(e) => {
                    app.status = format!("SAVE FAILED: {e}. Your changes remain in memory.");
                }
            }
            app.save_requested = false;
        } else if app.save_requested {
            app.save_requested = false;
            app.status = "Demo session: changes stay in memory; no files written.".into();
        }
        if let Some(target) = app.open_config_requested.take()
            && let Err(error) = edit_configuration(&mut terminal, &mut app, target)
        {
            app.status = format!("Editor failed: {error}");
        }
        terminal.draw(|frame| ui::draw(frame, &mut app))?;
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => app.key(key),
                Event::Paste(text) => app.paste(&text),
                Event::Mouse(mouse)
                    if (app.modal.is_none()
                        || matches!(
                            app.modal,
                            Some(app::Modal::SelectNotebook(_) | app::Modal::SummaryPlan { .. })
                        ))
                        && (!app.editing || app.tab == app::Tab::Chat) =>
                {
                    match mouse.kind {
                        MouseEventKind::Down(event::MouseButton::Left) => {
                            let action = app
                                .hits
                                .iter()
                                .rev()
                                .find(|(rect, _)| {
                                    rect.contains(Position::new(mouse.column, mouse.row))
                                })
                                .map(|(_, action)| *action);
                            if let Some(action) = action {
                                let search_source = matches!(action, Action::Source(_));
                                app.action(action);
                                if search_source {
                                    app.status = "x selects a source · Enter opens it".into();
                                }
                            }
                        }
                        MouseEventKind::ScrollDown => app.move_selection(true),
                        MouseEventKind::ScrollUp => app.move_selection(false),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
    if !demo {
        app.library.save(&path)?;
    }
    Ok(())
}

fn edit_configuration(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    target: EditorTarget,
) -> io::Result<()> {
    let argv = app.editor.argv()?;
    let path = match target {
        EditorTarget::Config => {
            if !app.config_path.try_exists()? {
                config::initialize(&app.config_path)?;
            }
            app.config_path.clone()
        }
        EditorTarget::Theme => config::editable_theme(&app.config_path, &app.theme_name)?,
    };
    execute!(
        io::stdout(),
        PopKeyboardEnhancementFlags,
        DisableMouseCapture,
        DisableBracketedPaste,
        Show,
        LeaveAlternateScreen
    )?;
    disable_raw_mode()?;
    let result = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .arg(&path)
        .status();
    // Restore Kaiser even when the editor could not be started or exited with an error.
    enable_raw_mode()?;
    execute!(
        io::stdout(),
        EnterAlternateScreen,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES),
        EnableMouseCapture,
        EnableBracketedPaste,
        Hide
    )?;
    // Reset the full-screen buffers without a cursor-position query. Some terminals
    // cannot answer that query immediately after handing control back from an editor.
    terminal.resize(terminal.size()?.into())?;
    let status = result?;
    if !status.success() {
        return Err(io::Error::other(format!("editor exited with {status}")));
    }
    let reload = match target {
        EditorTarget::Config => "config-reload",
        EditorTarget::Theme => "theme-reload",
    };
    app.status = format!("Editor closed · :{reload} applies your changes");
    Ok(())
}
