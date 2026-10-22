//! Workspace rendering.
use super::{
    Action, App, Constraint, Focus, Frame, Layout, Line, Modifier, Paragraph, Rect, Span, Style,
    Tab, body, bold, chat, empty_state, fit_status, inset, panel, startup, status_style, style,
    wrapped,
};
use unicode_width::UnicodeWidthChar;

pub(super) fn main_area(frame: &mut Frame, app: &mut App, area: Rect) {
    if !app.notebook_selected {
        startup(frame, app, area);
        return;
    }
    if app.focus == Focus::Flashcards && (app.tab == Tab::Chat || app.maximized || area.height < 23)
    {
        flashcards_panel(frame, app, area);
        return;
    }
    let flashcards_height = if app.tab == Tab::Chat || app.maximized || area.height < 23 {
        0
    } else {
        (area.height / 3).clamp(7, 11)
    };
    let rows = Layout::vertical([Constraint::Min(8), Constraint::Length(flashcards_height)])
        .spacing(if flashcards_height > 0 { 1 } else { 0 })
        .split(area);
    workspace(frame, app, rows[0]);
    if flashcards_height > 0 {
        flashcards_panel(frame, app, rows[1]);
    }
}

pub(super) fn workspace(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme;
    let inner = panel(frame, app, area, "Workspace", Focus::Workspace);
    let mut x = inner.x + 1;
    for (tab, label) in [
        (Tab::Chat, "Chat"),
        (Tab::Cards, "Flashcards"),
        (Tab::Notes, "Notes"),
    ] {
        let width = label.len() as u16 + 3;
        if x + width > inner.right() {
            break;
        }
        let rect = Rect::new(x, inner.y, width, 1);
        frame.render_widget(
            Paragraph::new(format!(" {label} ")).style(if app.tab == tab {
                bold(theme.accent).bg(theme.selection)
            } else {
                style(theme.muted)
            }),
            rect,
        );
        app.hits.push((rect, Action::Tab(tab)));
        x += width;
    }
    let content = Rect::new(
        inner.x + 1,
        inner.y + 2,
        inner.width.saturating_sub(2),
        inner.height.saturating_sub(2),
    );
    match app.tab {
        Tab::Chat => chat(frame, app, content),
        Tab::Cards => cards(frame, app, content),
        Tab::Notes => notes_view(frame, app, content),
    }
}

pub(super) fn notes_view(frame: &mut Frame, app: &mut App, area: Rect) {
    use crate::notes::Mode;
    let theme = app.theme;
    if app.notes.preview && !app.editing {
        if app.library.book().notes.trim().is_empty() {
            empty_state(
                frame,
                area,
                "No notes yet",
                &format!(
                    "i to write Markdown · {} p toggles preview",
                    app.leader_label()
                ),
                theme,
            );
            app.scroll_max = 0;
        } else {
            let mut lines = vec![
                Line::styled(
                    format!("MARKDOWN · i edit · {} p raw editor", app.leader_label()),
                    style(theme.muted),
                ),
                Line::raw(""),
            ];
            lines.extend(crate::markdown::render(
                &app.library.book().notes,
                area.width,
                theme,
            ));
            body(frame, app, area, lines, false);
        }
        return;
    }
    if area.height < 2 || area.width < 6 {
        return;
    }
    let text = &app.library.book().notes;
    let (cursor_row, cursor_column) = app.notes.position(text);
    let mode = if app.editing {
        "INSERT"
    } else if app.notes.mode == Mode::Select {
        "SELECT"
    } else {
        "NORMAL"
    };
    frame.render_widget(
        Paragraph::new(format!(
            "{mode} · {}:{} · {} p Markdown preview",
            cursor_row + 1,
            cursor_column + 1,
            app.leader_label()
        ))
        .style(style(theme.muted)),
        Rect::new(area.x, area.y, area.width, 1),
    );
    let content = Rect::new(area.x, area.y + 1, area.width, area.height - 1);
    let rows: Vec<&str> = text.split('\n').collect();
    let scroll = app
        .scroll
        .unwrap_or(0)
        .min(rows.len().saturating_sub(content.height as usize));
    let scroll = if cursor_row < scroll {
        cursor_row
    } else if cursor_row >= scroll + content.height as usize {
        cursor_row + 1 - content.height as usize
    } else {
        scroll
    };
    app.scroll = Some(scroll);
    app.scroll_max = rows.len().saturating_sub(content.height as usize);
    let gutter = (rows.len().to_string().len() as u16 + 2).max(4);
    let available = content.width.saturating_sub(gutter);
    let cursor_cells = rows.get(cursor_row).map_or(0, |line| {
        line.chars()
            .take(cursor_column)
            .map(|c| if c == '\t' { 4 } else { c.width().unwrap_or(0) })
            .sum::<usize>()
    });
    let horizontal = cursor_cells.saturating_sub(available.saturating_sub(1) as usize);
    let selection = app.notes.selection(text);
    let mut index = rows
        .iter()
        .take(scroll)
        .map(|line| line.chars().count() + 1)
        .sum::<usize>();
    for (offset, line) in rows
        .iter()
        .skip(scroll)
        .take(content.height as usize)
        .enumerate()
    {
        let y = content.y + offset as u16;
        frame.render_widget(
            Paragraph::new(format!(
                "{:>width$} ",
                scroll + offset + 1,
                width = gutter.saturating_sub(1) as usize
            ))
            .style(style(theme.muted)),
            Rect::new(content.x, y, gutter, 1),
        );
        let mut spans = vec![];
        for (column, c) in line.chars().enumerate() {
            let selected = !app.editing && selection.contains(&(index + column));
            spans.push(Span::styled(
                if c == '\t' {
                    "    ".into()
                } else {
                    c.to_string()
                },
                if selected {
                    style(theme.foreground).add_modifier(Modifier::REVERSED)
                } else {
                    style(theme.foreground)
                },
            ));
        }
        if scroll + offset == cursor_row && cursor_column >= line.chars().count() && !app.editing {
            spans.push(Span::styled(
                " ",
                style(theme.foreground).add_modifier(Modifier::REVERSED),
            ));
        }
        frame.render_widget(
            Paragraph::new(Line::from(spans)).scroll((0, horizontal.min(u16::MAX as usize) as u16)),
            Rect::new(content.x + gutter, y, available, 1),
        );
        index += line.chars().count() + 1;
    }
    if app.editing && available > 0 {
        frame.set_cursor_position((
            content.x
                + gutter
                + cursor_cells
                    .saturating_sub(horizontal)
                    .min(available as usize - 1) as u16,
            content.y + cursor_row.saturating_sub(scroll) as u16,
        ));
    }
}

pub(super) fn source_lines(app: &App, width: u16) -> Vec<Line<'static>> {
    let theme = app.theme;
    if let Some(source) = app.library.book().sources.get(app.source) {
        let mut lines = vec![
            Line::styled(source.name.clone(), bold(theme.foreground)),
            Line::raw(""),
        ];
        for (index, line) in source.content.lines().enumerate() {
            let parts = textwrap::wrap(line, usize::from(width.saturating_sub(6).max(1)));
            if parts.is_empty() {
                lines.push(Line::raw(""));
            }
            for (part, value) in parts.into_iter().enumerate() {
                lines.push(Line::from(vec![
                    Span::styled(
                        if part == 0 {
                            format!("{:>3}  ", index + 1)
                        } else {
                            "     ".into()
                        },
                        style(theme.muted),
                    ),
                    Span::styled(value.into_owned(), style(theme.foreground)),
                ]));
            }
        }
        lines
    } else {
        vec![Line::styled("No source to display", style(theme.muted))]
    }
}

/// Wrap the composer without dropping spaces or a trailing newline.
pub(super) fn cards(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme;
    let Some(card) = app.current_card().cloned() else {
        empty_state(
            frame,
            area,
            "No flashcards yet",
            &format!(
                "{} m to make a card · :generate-cards from sources",
                app.leader_label()
            ),
            theme,
        );
        return;
    };
    let mut lines = vec![
        Line::styled(
            format!(
                "{}Flashcard {} / {}",
                if card.draft { "Draft · " } else { "" },
                app.card + 1,
                app.cards().len()
            ),
            style(theme.muted),
        ),
        Line::raw(""),
    ];
    lines.extend(wrapped(&card.front, area.width, theme.accent));
    lines.push(Line::raw(""));
    if app.revealed {
        lines.push(Line::styled("ANSWER / DEFINITION", bold(theme.success)));
        lines.push(Line::raw(""));
        lines.extend(wrapped(&card.back, area.width, theme.foreground));
        if let Some(citation) = &card.citation {
            lines.push(Line::raw(""));
            lines.extend(wrapped(
                &format!("Source: {} · line {}", citation.source_name, citation.line),
                area.width,
                theme.muted,
            ));
        }
    } else {
        lines.push(Line::styled("Enter  show answer", style(theme.muted)));
    }
    let content = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));
    body(frame, app, content, lines, false);
    frame.render_widget(
        Paragraph::new("←/→ previous/next · Enter flip · e edit").style(status_style()),
        Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1),
    );
}

pub(super) fn flashcards_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme;
    let count = app.library.book().cards.len();
    let inner = panel(
        frame,
        app,
        area,
        &format!("Flashcards · {count}"),
        Focus::Flashcards,
    );
    let content = inset(inner, 1, 0);
    if content.is_empty() {
        return;
    }
    let list_area = Rect::new(
        content.x,
        content.y,
        content.width,
        content.height.saturating_sub(1),
    );
    if count == 0 {
        empty_state(
            frame,
            list_area,
            "No flashcards yet",
            &format!("{} m to make a card", app.leader_label()),
            theme,
        );
    } else {
        app.flashcard = app.flashcard.min(count - 1);
        let capacity = list_area.height as usize;
        let start = app.flashcard.saturating_sub(capacity.saturating_sub(1));
        for (index, card) in app
            .library
            .book()
            .cards
            .iter()
            .enumerate()
            .skip(start)
            .take(capacity)
        {
            let selected = index == app.flashcard;
            let row = Rect::new(
                list_area.x,
                list_area.y + (index - start) as u16,
                list_area.width,
                1,
            );
            let question = fit_status(&card.front, list_area.width.saturating_sub(2) as usize);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(if selected { "▸ " } else { "  " }, style(theme.accent)),
                    Span::styled(question, style(theme.foreground)),
                ]))
                .style(Style::default().bg(
                    if selected && app.focus == Focus::Flashcards {
                        theme.selection
                    } else {
                        theme.panel
                    },
                )),
                row,
            );
            app.hits.push((row, Action::SelectFlashcard(index)));
        }
    }
    frame.render_widget(
        Paragraph::new(format!(
            "↑↓ choose · Enter open · e edit · {} m make · {} F browse",
            app.leader_label(),
            app.leader_label()
        ))
        .style(status_style()),
        Rect::new(content.x, content.bottom() - 1, content.width, 1),
    );
}
