//! Dialogs rendering.
use super::{
    Action, App, COMMANDS, Clear, Constraint, Frame, LEADER_BINDINGS, Layout, Line, Modal,
    Modifier, Paragraph, Rect, Span, block, bold, fit_status, inset, source_lines, status_style,
    style, wrapped,
};
use unicode_width::UnicodeWidthStr;

pub(super) fn ex_menu(frame: &mut Frame, app: &App, line: &str) {
    let theme = app.theme;
    let screen = frame.area();
    if screen.width < 20 || screen.height < 6 {
        return;
    }
    let matches = app.ex_menu_suggestions(line);
    let height = (matches.len().max(1) as u16 + 3).min(screen.height.saturating_sub(3));
    let x = screen.x + if screen.width >= 90 { 10 } else { 1 };
    let width = screen.right().saturating_sub(x + 1).min(76);
    let area = Rect::new(x, screen.bottom() - 1 - height, width, height);
    frame.render_widget(Clear, area);
    let title = if line.trim_start().starts_with("theme ") {
        "Themes"
    } else {
        "Commands"
    };
    let widget = block(format!("{title} · {}", matches.len()), true, theme);
    let inner = widget.inner(area);
    frame.render_widget(widget, area);
    let capacity = inner.height.saturating_sub(1) as usize;
    let selected = app.ex_selected.filter(|i| *i < matches.len());
    let start = selected
        .unwrap_or(0)
        .saturating_sub(capacity.saturating_sub(1));
    for (position, suggestion) in matches.iter().enumerate().skip(start).take(capacity) {
        let active = selected == Some(position);
        let row = Rect::new(inner.x, inner.y + (position - start) as u16, inner.width, 1);
        let columns = Layout::horizontal([
            Constraint::Length(20.min(inner.width / 2)),
            Constraint::Min(0),
        ])
        .split(row);
        let bg = if active { theme.selection } else { theme.panel };
        frame.render_widget(
            Paragraph::new(format!(
                "{} {}",
                if active { "▸" } else { " " },
                suggestion.label
            ))
            .style(bold(theme.accent).bg(bg)),
            columns[0],
        );
        frame.render_widget(
            Paragraph::new(suggestion.description.clone()).style(
                style(if active {
                    theme.foreground
                } else {
                    theme.muted
                })
                .bg(bg),
            ),
            columns[1],
        );
    }
    if matches.is_empty() {
        frame.render_widget(
            Paragraph::new(" No matching commands").style(style(theme.muted)),
            inner,
        );
    }
    frame.render_widget(
        Paragraph::new(if line.trim_start().starts_with("theme ") {
            " Tab/Shift+Tab preview · Enter keep · Esc restore"
        } else {
            " ↑↓ choose · Tab/Shift+Tab cycle · Enter run · Esc cancel"
        })
        .style(status_style()),
        Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
    );
}

pub(super) fn notebook_picker(frame: &mut Frame, app: &mut App, query: &str) {
    let theme = app.theme;
    let screen = frame.area();
    let width = screen.width.saturating_sub(6).min(140);
    let height = screen.height.saturating_sub(4).min(32);
    let area = Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, area);
    let show_preview = width >= 70 && height >= 10;
    let columns = Layout::horizontal(if show_preview {
        vec![Constraint::Percentage(38), Constraint::Percentage(62)]
    } else {
        vec![Constraint::Percentage(100)]
    })
    .split(area);
    let matches = app.notebook_matches(query);
    let selected = app.modal_index.min(matches.len().saturating_sub(1));
    let title = format!(
        "Notebooks · {}/{}",
        if matches.is_empty() { 0 } else { selected + 1 },
        matches.len()
    );
    let widget = block(title, true, theme);
    let inner = inset(widget.inner(columns[0]), 1, 0);
    frame.render_widget(widget, columns[0]);
    let rows = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner);
    let capacity = rows[0].height as usize;
    let start = selected.saturating_sub(capacity.saturating_sub(1));
    if matches.is_empty() {
        frame.render_widget(
            Paragraph::new(if app.library.notebooks.is_empty() {
                "No notebooks yet"
            } else {
                "No matching notebooks"
            })
            .style(style(theme.muted)),
            rows[0],
        );
    }
    for (position, &index) in matches.iter().enumerate().skip(start).take(capacity) {
        let row = Rect::new(
            rows[0].x,
            rows[0].y + (position - start) as u16,
            rows[0].width,
            1,
        );
        let active = position == selected;
        let label = format!(
            "{} {}",
            if active { "▸" } else { " " },
            app.library.notebooks[index].name
        );
        frame.render_widget(
            Paragraph::new(fit_status(&label, row.width as usize)).style(if active {
                bold(theme.accent).bg(theme.selection)
            } else {
                style(theme.foreground)
            }),
            row,
        );
        app.hits.push((row, Action::Notebook(index)));
    }
    frame.render_widget(
        Paragraph::new("─".repeat(rows[1].width as usize)).style(style(theme.border)),
        rows[1],
    );
    let prompt = format!("› {query}▏");
    frame.render_widget(
        Paragraph::new(prompt.clone())
            .style(style(theme.accent))
            .scroll((
                0,
                prompt.width().saturating_sub(rows[2].width as usize) as u16,
            )),
        rows[2],
    );
    if !show_preview {
        return;
    }
    let notebook = matches
        .get(selected)
        .map(|&index| &app.library.notebooks[index]);
    let widget = block(
        notebook
            .map(|book| book.name.clone())
            .unwrap_or_else(|| "Preview".into()),
        false,
        theme,
    );
    let inner = inset(widget.inner(columns[1]), 1, 1);
    frame.render_widget(widget, columns[1]);
    let Some(book) = notebook else {
        frame.render_widget(
            Paragraph::new("No notebook selected").style(style(theme.muted)),
            inner,
        );
        return;
    };
    let mut lines = wrapped(
        &book.directory.display().to_string(),
        inner.width,
        theme.muted,
    );
    lines.push(Line::raw(""));
    lines.extend(wrapped(
        &format!(
            "{} sources · {} flashcards · {} messages",
            book.sources.len(),
            book.flashcard_count(),
            book.messages.len()
        ),
        inner.width,
        theme.muted,
    ));
    lines.push(Line::raw(""));
    if !book.notes.trim().is_empty() {
        // Bound preview work independently of the size of the saved notebook.
        let excerpt: String = book.notes.chars().take(8000).collect();
        lines.extend(crate::markdown::render(&excerpt, inner.width, theme));
        lines.push(Line::raw(""));
    } else if !book.cards.is_empty() {
        lines.push(Line::styled("Flashcards", bold(theme.accent)));
        for card in book.cards.iter().filter(|card| !card.draft).take(6) {
            lines.push(Line::styled(
                fit_status(&card.front, inner.width as usize),
                style(theme.foreground),
            ));
        }
        lines.push(Line::raw(""));
    } else if let Some(message) = book.messages.last() {
        lines.push(Line::styled("Chat", bold(theme.accent)));
        let excerpt: String = message.text.chars().take(8000).collect();
        lines.extend(crate::markdown::render(&excerpt, inner.width, theme));
        lines.push(Line::raw(""));
    }
    if !book.sources.is_empty() {
        lines.push(Line::styled("Sources", bold(theme.accent)));
        for source in book.sources.iter().take(inner.height as usize) {
            lines.push(Line::styled(
                fit_status(&format!("  {}", source.name), inner.width as usize),
                style(theme.foreground),
            ));
        }
    } else if book.notes.trim().is_empty() && book.cards.is_empty() && book.messages.is_empty() {
        lines.push(Line::styled("This notebook is empty.", style(theme.muted)));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

pub(super) fn leader_column_width() -> usize {
    LEADER_BINDINGS
        .iter()
        .map(|(_, label)| label.width() + 3)
        .max()
        .unwrap_or(0)
}

pub(super) fn modal_view(frame: &mut Frame, app: &mut App, modal: Modal) {
    if let Modal::Ex(line) = &modal {
        ex_menu(frame, app, line);
        return;
    }
    if let Modal::SelectNotebook(query) = &modal {
        notebook_picker(frame, app, query);
        return;
    }
    let theme = app.theme;
    let screen = frame.area();
    let desired_width = if matches!(modal, Modal::Leader) {
        (leader_column_width() * 2 + 9) as u16
    } else {
        86
    };
    let width = (screen.width.saturating_sub(8)).min(desired_width);
    let height = match modal {
        Modal::Help | Modal::Reader | Modal::AiUsage(_) => 30,
        Modal::Accounts | Modal::ChatgptModels | Modal::SignIn(_) => 19,
        Modal::Card { .. } => 24,
        Modal::DeleteNotebook { .. } | Modal::SummaryPlan { .. } => 14,
        Modal::Palette(_) | Modal::Search(_) | Modal::SelectNotebook(_) | Modal::Import(_) => 19,
        Modal::Leader => LEADER_BINDINGS.len().div_ceil(2) as u16 + 4,
        _ => 10,
    }
    .min(screen.height.saturating_sub(4));
    let area = Rect::new(
        screen.x + (screen.width - width) / 2,
        screen.y + (screen.height - height) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, area);
    let title = match &modal {
        Modal::SummaryPlan { cards: Some(_), .. } => "Generate flashcards",
        Modal::SummaryPlan { .. } => "Summary plan",
        Modal::AiUsage(_) => "AI usage · local",
        Modal::Accounts => "ChatGPT accounts",
        Modal::ChatgptModels => "ChatGPT models",
        Modal::SignIn(_) => "Continue with ChatGPT",
        Modal::ChatgptWelcome => "You're using your ChatGPT plan",
        Modal::Help => "Field guide",
        Modal::Reader => "Source reader",
        Modal::Ex(_) => unreachable!(),
        Modal::Leader => "Navigation",
        Modal::Import(_) => "Import a source",
        Modal::Notebook(_) => "New notebook",
        Modal::SelectNotebook(_) => "Select notebook",
        Modal::DeleteNotebook { .. } => "Delete notebook?",
        Modal::Search(_) => "Search this notebook",
        Modal::Palette(_) => "Command palette",
        Modal::Card { .. } => "Shape a study card",
    };
    let widget = block(title.into(), true, theme);
    let inner = inset(widget.inner(area), 2, 1);
    frame.render_widget(widget, area);
    match modal {
        Modal::SummaryPlan {
            cards,
            requests,
            label,
            bytes,
            ..
        } => {
            let detail = cards
                .as_ref()
                .map(|request| format!("Up to {} editable flashcard drafts.", request.count))
                .unwrap_or_else(|| "Includes all parts and final synthesis.".into());
            let text = format!(
                "{label}\n\n{bytes} bytes · up to {requests} AI requests\n{detail}\nCache hits may reduce requests.\nDaily limits apply."
            );
            frame.render_widget(
                Paragraph::new(wrapped(&text, inner.width, theme.foreground)),
                Rect::new(
                    inner.x,
                    inner.y,
                    inner.width,
                    inner.height.saturating_sub(2),
                ),
            );
            let buttons =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(
                    Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1),
                );
            for (index, label) in [
                " Cancel ",
                if cards.is_some() {
                    " Generate cards "
                } else {
                    " Generate summary "
                },
            ]
            .iter()
            .enumerate()
            {
                frame.render_widget(
                    Paragraph::new(*label)
                        .alignment(ratatui::layout::Alignment::Center)
                        .style(if app.modal_index == index {
                            bold(theme.accent).bg(theme.selection)
                        } else {
                            style(theme.muted)
                        }),
                    buttons[index],
                );
                app.hits
                    .push((buttons[index], Action::ConfirmSummary(index == 1)));
            }
        }
        Modal::AiUsage(text) => {
            let lines = wrapped(&text, inner.width, theme.foreground);
            app.modal_index = app
                .modal_index
                .min(lines.len().saturating_sub(inner.height as usize));
            frame.render_widget(
                Paragraph::new(lines.into_iter().skip(app.modal_index).collect::<Vec<_>>()),
                inner,
            );
        }
        Modal::SignIn(url) => {
            let text = if url.is_empty() {
                "Preparing sign-in…".to_owned()
            } else {
                format!("Continue in your browser.\n\n{url}")
            };
            frame.render_widget(
                Paragraph::new(wrapped(&text, inner.width, theme.foreground)),
                inner,
            );
        }
        Modal::ChatgptWelcome => {
            let text = format!(
                "Eligible AI requests in Kaiser use your ChatGPT plan or credits.\n\nManage usage in ChatGPT settings:\n{}\n\nGot it",
                crate::auth::USAGE_URL
            );
            frame.render_widget(
                Paragraph::new(wrapped(&text, inner.width, theme.foreground)),
                inner,
            );
        }
        Modal::Accounts | Modal::ChatgptModels => {
            let labels: Vec<String> = if matches!(modal, Modal::Accounts) {
                app.accounts
                    .iter()
                    .enumerate()
                    .map(|(index, account)| {
                        format!(
                            "{}{} · {} · {}",
                            if account.active { "● " } else { "" },
                            account.email,
                            index + 1,
                            if !account.connected {
                                "signed out"
                            } else if account.plan {
                                "plan enabled"
                            } else {
                                "plan disabled"
                            }
                        )
                    })
                    .chain(std::iter::once("Continue with ChatGPT".into()))
                    .collect()
            } else {
                app.chatgpt_models
                    .iter()
                    .map(|model| model.name.clone())
                    .collect()
            };
            let capacity = inner.height as usize;
            app.modal_index = app.modal_index.min(labels.len().saturating_sub(1));
            let start = app.modal_index.saturating_sub(capacity.saturating_sub(1));
            let mut lines = Vec::new();
            if labels.is_empty() {
                lines.push(Line::styled("Loading models…", style(theme.muted)));
            }
            for (index, label) in labels.into_iter().enumerate().skip(start).take(capacity) {
                let selected = index == app.modal_index;
                lines.push(Line::styled(
                    format!("{} {label}", if selected { "▸" } else { " " }),
                    if selected {
                        bold(theme.accent).bg(theme.selection)
                    } else {
                        style(theme.foreground)
                    },
                ));
            }
            frame.render_widget(Paragraph::new(lines), inner);
        }
        Modal::Ex(_) => unreachable!(),
        Modal::Reader => {
            let lines = source_lines(app, inner.width);
            app.modal_index = app
                .modal_index
                .min(lines.len().saturating_sub(inner.height as usize));
            frame.render_widget(
                Paragraph::new(lines.into_iter().skip(app.modal_index).collect::<Vec<_>>()),
                inner,
            );
        }
        Modal::Help => {
            let prefix = app.leader_label();
            let text = format!(
                "{prefix} + key    Open leader navigation (sequential presses)\nEsc              Cancel the pending leader sequence\n{prefix} c / F      Chat / Flashcards\n{prefix} o / d      Notes / Drafts\n<super> + f      Select notebooks\n<super> + n      Make new notebooks\n<super> + h      Home (also {prefix} h)\n<super> + ?      Keybindings\n{prefix} s          Sources\n{prefix} w / e      Workspace / Flashcards panel\n{prefix} f / n      Select notebook / New notebook\n{prefix} a / m      Import / Make card\n{prefix} / / P      Search / Commands\n{prefix} z / B      Maximize / Toggle sidebar\n{prefix} Q          Save and quit\n\nLOCAL CONTROLS\nTab / Shift+Tab  Cycle panels / menu choices\n↑ ↓ or j k       Navigate lists / scroll\ni / Esc          Write / return to navigation\nx / Enter        Toggle source / read source\n← → / Enter      Previous/next flashcard / flip\nCtrl+S / Ctrl+A  Save card / approve (in editor)\n↑ ↓ / Enter      Choose / open a card in Flashcards panel\nPgUp PgDn        Scroll workspace · Home / End to jump\nCtrl+C           Save and quit\n:                Command line · :q quits · :w saves\n\nNOTES (core Helix keys)\ni a I A o O     Insert / append / open line\nh j k l / w b e  Move cursor / words\ngg ge gh gl      Document / line bounds\nv x %           Select / line / all\nd c y p P       Delete / change / yank / paste\nu U / Esc       Undo / redo / normal mode\n{prefix} p          Markdown preview / raw editor\n\nChange [keys] leader in config.toml.\n↑/↓ scroll this guide · Esc closes."
            );
            let lines = wrapped(&text, inner.width, theme.foreground);
            let offset = app
                .modal_index
                .min(lines.len().saturating_sub(inner.height as usize));
            frame.render_widget(
                Paragraph::new(lines.into_iter().skip(offset).collect::<Vec<_>>()),
                inner,
            );
        }
        Modal::Leader => {
            let mut lines = Vec::new();
            for pair in LEADER_BINDINGS.chunks(2) {
                let left = format!("{}  {}", pair[0].0, pair[0].1);
                let right = pair
                    .get(1)
                    .map(|(key, label)| format!("{key}  {label}"))
                    .unwrap_or_default();
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{left:<width$}   ", width = leader_column_width()),
                        style(theme.accent),
                    ),
                    Span::styled(right, style(theme.foreground)),
                ]));
            }
            let offset = app
                .modal_index
                .min(lines.len().saturating_sub(inner.height as usize));
            frame.render_widget(
                Paragraph::new(lines.into_iter().skip(offset).collect::<Vec<_>>()),
                inner,
            );
        }
        Modal::DeleteNotebook { notebook, .. } => {
            if let Some(notebook) = app.library.notebooks.get(notebook) {
                let mut lines = vec![
                    Line::styled(
                        format!("Delete \"{}\"?", notebook.name),
                        bold(theme.foreground),
                    ),
                    Line::raw(""),
                ];
                lines.extend(wrapped(&format!("{} sources · {} flashcards · {} messages\n{} lines of notes\n\nPermanently removes notebook data.\nOriginal source files stay untouched.", notebook.sources.len(), notebook.cards.len(), notebook.messages.len(), notebook.notes.lines().count()), inner.width, theme.muted));
                frame.render_widget(
                    Paragraph::new(lines),
                    Rect::new(
                        inner.x,
                        inner.y,
                        inner.width,
                        inner.height.saturating_sub(3),
                    ),
                );
                let buttons =
                    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(Rect::new(
                            inner.x,
                            inner.bottom().saturating_sub(2),
                            inner.width,
                            1,
                        ));
                for (index, label) in [" Keep notebook ", " Delete notebook "]
                    .into_iter()
                    .enumerate()
                {
                    frame.render_widget(
                        Paragraph::new(label).style(if app.modal_index == index {
                            bold(theme.foreground).add_modifier(Modifier::REVERSED)
                        } else {
                            style(theme.muted)
                        }),
                        buttons[index],
                    );
                    app.hits
                        .push((buttons[index], Action::ConfirmNotebookDeletion(index == 1)));
                }
                frame.render_widget(
                    Paragraph::new("y delete · n/Esc keep · Tab + Enter").style(status_style()),
                    Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1),
                );
            }
        }
        Modal::SelectNotebook(_) => unreachable!(),
        Modal::Card {
            front, back, field, ..
        } => {
            let rows = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(5),
                Constraint::Min(3),
                Constraint::Length(2),
            ])
            .spacing(1)
            .split(inner);
            frame.render_widget(
                Paragraph::new("One clear question. A concise answer. Keep the source attached.")
                    .style(style(theme.muted)),
                rows[0],
            );
            let front_block = block("Question".into(), field == 0, theme);
            let back_block = block("Answer".into(), field == 1, theme);
            let front_inner = front_block.inner(rows[1]);
            let back_inner = back_block.inner(rows[2]);
            let front_lines = wrapped(&front, front_inner.width, theme.foreground);
            let back_lines = wrapped(&back, back_inner.width, theme.foreground);
            frame.render_widget(
                Paragraph::new(front_lines.clone())
                    .block(front_block)
                    .scroll((
                        front_lines
                            .len()
                            .saturating_sub(front_inner.height as usize)
                            as u16,
                        0,
                    )),
                rows[1],
            );
            frame.render_widget(
                Paragraph::new(back_lines.clone())
                    .block(back_block)
                    .scroll((
                        back_lines.len().saturating_sub(back_inner.height as usize) as u16,
                        0,
                    )),
                rows[2],
            );
            frame.render_widget(Paragraph::new("Tab field · Ctrl+U clear field · Esc cancel\nCtrl+S save draft / edits · Ctrl+A finish card").style(status_style()), rows[3]);
        }
        Modal::Import(ref text)
        | Modal::Notebook(ref text)
        | Modal::Search(ref text)
        | Modal::Palette(ref text) => {
            let mut lines = vec![
                Line::styled(format!("› {text}▏"), style(theme.accent)),
                Line::raw(""),
            ];
            if matches!(modal, Modal::Import(_)) {
                match app.import_matches(text) {
                    Ok(entries) => {
                        let capacity = inner.height.saturating_sub(2) as usize;
                        app.modal_index = app.modal_index.min(entries.len().saturating_sub(1));
                        let start = app.modal_index.saturating_sub(capacity.saturating_sub(1));
                        if entries.is_empty() {
                            lines.push(Line::styled("No matching files", style(theme.muted)));
                        }
                        for (index, entry) in entries.iter().enumerate().skip(start).take(capacity)
                        {
                            let selected = index == app.modal_index;
                            lines.push(Line::styled(
                                format!("{} {}", if selected { "▸" } else { " " }, entry.label),
                                if selected {
                                    bold(theme.accent).bg(theme.selection)
                                } else {
                                    style(theme.foreground)
                                },
                            ));
                        }
                    }
                    Err(error) => lines.push(Line::styled(error, style(theme.muted))),
                }
            }
            if matches!(modal, Modal::Palette(_)) {
                let start = app.modal_index.saturating_sub(8);
                for (i, command) in COMMANDS
                    .iter()
                    .filter(|c| c.label().to_lowercase().contains(&text.to_lowercase()))
                    .enumerate()
                    .skip(start)
                    .take(9)
                {
                    lines.push(Line::styled(
                        format!(
                            "{} {}",
                            if i == app.modal_index { "▸" } else { " " },
                            command.label()
                        ),
                        if i == app.modal_index {
                            bold(theme.accent).bg(theme.selection)
                        } else {
                            style(theme.foreground)
                        },
                    ));
                }
            }
            if matches!(modal, Modal::Search(_)) {
                let results = app.search(text);
                if results.is_empty() && !text.is_empty() {
                    lines.push(Line::styled("No matches yet.", style(theme.muted)));
                }
                let start = app.modal_index.saturating_sub(8);
                for (i, (label, _)) in results.into_iter().enumerate().skip(start).take(9) {
                    lines.push(Line::styled(
                        format!("{} {label}", if i == app.modal_index { "▸" } else { " " }),
                        if i == app.modal_index {
                            bold(theme.accent).bg(theme.selection)
                        } else {
                            style(theme.foreground)
                        },
                    ));
                }
            }
            frame.render_widget(Paragraph::new(lines), inner);
        }
    }
}
