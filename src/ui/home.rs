//! Home rendering.
use super::{
    Action, App, Block, BorderType, Borders, Command, Constraint, Frame, Layout, Line, Paragraph,
    Rect, Span, Style, Theme, bold, directory_label, fit_status, style,
};

pub(super) fn home_panel(frame: &mut Frame, area: Rect, title: &str, theme: Theme) -> Rect {
    let shell = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(style(theme.home_border.unwrap_or(theme.accent)))
        .style(Style::default().bg(theme.panel))
        .title(Line::styled(format!(" {title} "), style(theme.muted)));
    let inner = shell.inner(area);
    frame.render_widget(shell, area);
    inner
}

pub(super) fn startup(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme;
    let (welcome, notebooks) = if area.width >= 83 {
        let columns = Layout::horizontal([Constraint::Min(35), Constraint::Length(44)])
            .spacing(1)
            .split(area);
        (columns[0], columns[1])
    } else {
        let rows = Layout::vertical([Constraint::Length(8), Constraint::Min(6)])
            .spacing(1)
            .split(area);
        (rows[0], rows[1])
    };
    let inner = home_panel(frame, welcome, "Kaiser", theme);
    let roomy = inner.height >= 10;
    let text_height = if roomy { 7 } else { 5 };
    let logo_size = app.logo.as_ref().and_then(|logo| {
        let available = inner.height.saturating_sub(text_height + 1).min(14);
        (available >= 3).then(|| {
            logo.size_for(
                ratatui_image::Resize::Fit(None),
                ratatui::layout::Size::new(inner.width.saturating_sub(4).min(28), available),
            )
        })
    });
    let logo_height = logo_size.map_or(0, |size| size.height + 1);
    let top = inner.y + inner.height.saturating_sub(text_height + logo_height) / 2;
    if let (Some(size), Some(logo)) = (logo_size, app.logo.as_mut()) {
        frame.render_stateful_widget(
            ratatui_image::StatefulImage::default(),
            Rect::new(
                inner.x + inner.width.saturating_sub(size.width) / 2,
                top,
                size.width,
                size.height,
            ),
            logo,
        );
    }
    let title_y = top + logo_height;
    let mut introduction = vec![Line::styled("Kaiser", bold(theme.foreground))];
    if roomy {
        introduction.push(Line::styled(
            format!("v{}", env!("CARGO_PKG_VERSION")),
            style(theme.muted),
        ));
    }
    frame.render_widget(
        Paragraph::new(introduction).alignment(ratatui::layout::Alignment::Center),
        Rect::new(inner.x, title_y, inner.width, if roomy { 2 } else { 1 }),
    );
    let width = inner.width.saturating_sub(4).min(42);
    let x = inner.x + inner.width.saturating_sub(width) / 2;
    let y = title_y + if roomy { 4 } else { 2 };
    for (index, (label, key, action)) in [
        ("Select a notebook", "<super> + f", Action::PickNotebook),
        (
            "Make new notebooks",
            "<super> + n",
            Action::Command(Command::NewNotebook),
        ),
        ("Keybindings", "<super> + ?", Action::Command(Command::Help)),
    ]
    .into_iter()
    .enumerate()
    {
        let row = Rect::new(x, y + index as u16, width, 1);
        let label_width = width.saturating_sub(key.len() as u16 + 2);
        frame.render_widget(
            Paragraph::new(fit_status(label, label_width as usize)).style(if index == 0 {
                bold(theme.foreground)
            } else {
                style(theme.foreground)
            }),
            Rect::new(row.x, row.y, label_width, 1),
        );
        frame.render_widget(
            Paragraph::new(key)
                .style(style(theme.muted))
                .alignment(ratatui::layout::Alignment::Right),
            Rect::new(
                row.x + label_width + 1,
                row.y,
                row.width - label_width - 1,
                1,
            ),
        );
        app.hits.push((row, action));
    }
    notebooks_panel(frame, app, notebooks);
}

pub(super) fn notebooks_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme;
    let count = app.library.notebooks.len();
    let inner = home_panel(frame, area, &format!("Notebooks · {count}"), theme);
    let width = inner.width.saturating_sub(2);
    let x = inner.x + 1;
    let body_height = inner.height;
    let detailed = body_height >= 8;
    let item_height = if detailed { 4 } else { 1 };
    let visible = count.min((body_height / item_height) as usize);
    let count_width = 7;
    let name_width = width.saturating_sub(count_width);
    for (index, notebook) in app.library.notebooks.iter().take(visible).enumerate() {
        let top = inner.y + index as u16 * item_height;
        for offset in 0..item_height {
            let y = top + offset;
            let row = Rect::new(x, y, width, 1);
            let (prefix, label, value) = match offset {
                0 => (" ●  ", notebook.name.clone(), String::new()),
                1 => (" ├─ ", "Sources".into(), notebook.sources.len().to_string()),
                2 => (
                    " ├─ ",
                    "Flashcards".into(),
                    notebook.cards.len().to_string(),
                ),
                _ => (
                    " └─ ",
                    directory_label(&notebook.directory, name_width.saturating_sub(5)),
                    String::new(),
                ),
            };
            let text_style = if offset == 0 {
                bold(theme.foreground)
            } else {
                style(theme.muted)
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(prefix, style(theme.muted)),
                    Span::styled(
                        fit_status(&label, name_width.saturating_sub(5) as usize),
                        text_style,
                    ),
                ]))
                .style(Style::default().bg(theme.panel)),
                Rect::new(x, y, name_width, 1),
            );
            frame.render_widget(
                Paragraph::new(value).style(style(theme.foreground).bg(theme.panel)),
                Rect::new(x + name_width, y, count_width, 1),
            );
            app.hits.push((row, Action::Notebook(index)));
        }
    }
}
