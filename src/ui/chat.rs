//! Chat rendering.
use super::{
    Action, App, Block, BorderType, Borders, Constraint, Frame, Layout, Line, Paragraph, Rect,
    body, bold, empty_state, fit_status, inset, style, wrapped,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub(super) fn composer_lines(text: &str, width: u16) -> Vec<String> {
    let width = usize::from(width.max(1));
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut cells = 0;
    for c in text.chars() {
        if c == '\n' {
            lines.push(std::mem::take(&mut line));
            cells = 0;
            continue;
        }
        let size = c.width().unwrap_or(0);
        if cells + size > width && !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            cells = 0;
        }
        line.push(c);
        cells += size;
    }
    lines.push(line);
    if cells >= width {
        lines.push(String::new());
    }
    lines
}

pub(super) fn citation_groups(
    citations: &[crate::model::Citation],
    sources: &[crate::model::Source],
) -> Vec<(String, u64)> {
    let mut groups: Vec<(u64, String, Vec<usize>, Vec<String>)> = Vec::new();
    for (index, citation) in citations.iter().enumerate() {
        let location = sources
            .iter()
            .find(|source| source.id == citation.source_id)
            .map(|source| {
                let mut page = None;
                let mut first_page = None;
                for (index, line) in source
                    .content
                    .lines()
                    .take(citation.line.saturating_sub(1) + citation.passage.lines().count())
                    .enumerate()
                {
                    let label = line.trim().trim_start_matches('#').trim();
                    if label.starts_with("Page ") || label.starts_with("Slide ") {
                        page = Some(label.to_owned());
                    }
                    if index + 1 >= citation.line && first_page.is_none() {
                        first_page = page.clone();
                    }
                }
                match (first_page, page) {
                    (Some(first), Some(last)) if first != last => format!("{first} … {last}"),
                    (_, Some(last)) => last,
                    _ => format!("line {}", citation.line),
                }
            })
            .unwrap_or_else(|| format!("line {}", citation.line));
        if let Some(group) = groups
            .iter_mut()
            .find(|group| group.0 == citation.source_id && group.1 == citation.source_name)
        {
            group.2.push(index + 1);
            if group.3.last() != Some(&location) {
                group.3.push(location);
            }
        } else {
            groups.push((
                citation.source_id,
                citation.source_name.clone(),
                vec![index + 1],
                vec![location],
            ));
        }
    }
    groups
        .into_iter()
        .map(|(id, name, numbers, locations)| {
            let refs = if numbers.len() > 1 && numbers.windows(2).all(|pair| pair[1] == pair[0] + 1)
            {
                format!("{}–{}", numbers[0], numbers.last().unwrap())
            } else {
                numbers
                    .iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            };
            let place = if locations.len() > 1 {
                format!("{} … {}", locations[0], locations.last().unwrap())
            } else {
                locations[0].clone()
            };
            (format!("[{refs}] {name} · {place}"), id)
        })
        .collect()
}

pub(super) fn chat(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme;
    if area.is_empty() {
        return;
    }
    let input_lines = composer_lines(&app.input, area.width.saturating_sub(4));
    let composer_height = input_lines.len().clamp(1, 4) as u16 + 2;
    let rows = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(composer_height),
        Constraint::Length(1),
    ])
    .spacing(1)
    .split(area);
    let mut lines = Vec::new();
    let mut citations = Vec::new();
    if app.library.book().messages.is_empty() {
        empty_state(frame, rows[0], "No conversation yet", "", theme);
    }
    let question_width = (rows[0].width * 3 / 4).clamp(1, 72);
    for message in &app.library.book().messages {
        if message.role == "user" {
            lines.push(
                Line::styled("You", style(theme.muted))
                    .alignment(ratatui::layout::Alignment::Right),
            );
            for line in wrapped(
                &message.text,
                question_width.saturating_sub(2),
                theme.foreground,
            ) {
                lines.push(
                    Line::styled(
                        format!(" {line} "),
                        style(theme.foreground).bg(theme.selection),
                    )
                    .alignment(ratatui::layout::Alignment::Right),
                );
            }
        } else {
            lines.push(Line::styled("Kaiser", bold(theme.accent)));
            lines.extend(crate::markdown::render(&message.text, rows[0].width, theme));
            if !message.citations.is_empty() {
                lines.push(Line::raw(""));
            }
            for (label, source_id) in
                citation_groups(&message.citations, &app.library.book().sources)
            {
                if let Some(source) = app
                    .library
                    .book()
                    .sources
                    .iter()
                    .position(|source| source.id == source_id)
                {
                    citations.push((
                        lines.len(),
                        label.width().min(rows[0].width as usize) as u16,
                        source,
                    ));
                }
                lines.push(Line::styled(
                    fit_status(&label, rows[0].width as usize),
                    style(theme.success),
                ));
            }
        }
        lines.push(Line::raw(""));
    }
    if app
        .pending
        .as_ref()
        .is_some_and(|pending| pending.notebook == app.library.active)
    {
        lines.push(Line::styled("Kaiser", bold(theme.accent)));
        lines.push(Line::styled(
            if app.generation_progress.is_empty() {
                "Thinking…".to_owned()
            } else {
                app.generation_progress.clone()
            },
            style(theme.muted),
        ));
    }
    body(frame, app, rows[0], lines, true);
    let offset = app.scroll.unwrap_or(app.scroll_max).min(app.scroll_max);
    for (line, width, source) in citations {
        if line >= offset && line - offset < rows[0].height as usize {
            app.hits.push((
                Rect::new(rows[0].x, rows[0].y + (line - offset) as u16, width, 1),
                Action::ReadSource(source),
            ));
        }
    }
    let composer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(style(if app.editing {
            theme.accent
        } else {
            theme.border
        }));
    let input_area = inset(composer.inner(rows[1]), 1, 0);
    frame.render_widget(composer, rows[1]);
    app.hits.push((rows[1], Action::ComposeChat));
    let start = input_lines.len().saturating_sub(input_area.height as usize);
    if app.input.is_empty() {
        frame.render_widget(
            Paragraph::new("Ask about your sources…").style(style(theme.muted)),
            input_area,
        );
    } else {
        frame.render_widget(
            Paragraph::new(
                input_lines
                    .iter()
                    .skip(start)
                    .cloned()
                    .map(Line::raw)
                    .collect::<Vec<_>>(),
            )
            .style(style(theme.foreground)),
            input_area,
        );
    }
    if app.editing && app.modal.is_none() && !input_area.is_empty() {
        let last = input_lines.last().unwrap();
        frame.set_cursor_position((
            input_area.x + (last.width() as u16).min(input_area.width - 1),
            input_area.y + (input_lines.len() - 1 - start) as u16,
        ));
    }
    let send = Rect::new(
        rows[1].right().saturating_sub(8).max(rows[1].x),
        rows[1].bottom().saturating_sub(1),
        6.min(rows[1].width),
        1,
    );
    frame.render_widget(
        Paragraph::new(" Send ").style(
            style(if app.input.trim().is_empty() || app.pending.is_some() {
                theme.muted
            } else {
                theme.accent
            })
            .bg(theme.panel),
        ),
        send,
    );
    app.hits.push((send, Action::SendChat));
    let selected = app
        .library
        .book()
        .sources
        .iter()
        .filter(|source| source.selected)
        .count();
    let model = app
        .provider
        .as_ref()
        .map(|provider| {
            if provider.model.is_empty() {
                "Choose a model"
            } else {
                provider.model.as_str()
            }
        })
        .unwrap_or("Local source lookup");
    frame.render_widget(
        Paragraph::new(fit_status(
            &format!("{selected} sources · {model}"),
            rows[2].width as usize,
        ))
        .style(style(theme.muted)),
        rows[2],
    );
}
