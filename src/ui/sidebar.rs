//! Sidebar rendering.
use super::{
    Action, App, Constraint, Focus, Frame, Layout, Line, Paragraph, Rect, Span, Style, bold,
    empty_state, panel, status_style, style,
};

pub(super) fn sidebar(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = app.theme;
    let rows = Layout::vertical([Constraint::Length(3), Constraint::Min(5)])
        .spacing(1)
        .split(area);
    let inner = panel(frame, app, rows[0], "Notebook", Focus::Notebooks);
    let selected = if app.notebook_selected {
        app.library.book().name.clone()
    } else {
        "Select a notebook".into()
    };
    let parts = Layout::horizontal([Constraint::Min(1), Constraint::Length(3)]).split(inner);
    frame.render_widget(
        Paragraph::new(format!(" {selected}")).style(bold(theme.accent)),
        parts[0],
    );
    frame.render_widget(Paragraph::new(" ▾").style(style(theme.muted)), parts[1]);
    app.hits.push((rows[0], Action::PickNotebook));
    let source_count = if app.notebook_selected {
        app.library.book().sources.len()
    } else {
        0
    };
    let selected = if app.notebook_selected {
        app.library
            .book()
            .sources
            .iter()
            .filter(|s| s.selected)
            .count()
    } else {
        0
    };
    let inner = panel(
        frame,
        app,
        rows[1],
        &if app.notebook_selected {
            format!("Sources  {selected}/{source_count}")
        } else {
            "Sources".into()
        },
        Focus::Sources,
    );
    let visible = inner.height.saturating_sub(2) as usize;
    let start = app.source.saturating_sub(visible.saturating_sub(1));
    for (index, source) in app
        .library
        .book()
        .sources
        .iter()
        .enumerate()
        .skip(start)
        .take(if app.notebook_selected { visible } else { 0 })
    {
        let row = Rect::new(inner.x, inner.y + (index - start) as u16, inner.width, 1);
        let active = app.source == index && app.focus == Focus::Sources;
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    if source.selected { " ☑ " } else { " ☐ " },
                    style(if source.selected {
                        theme.success
                    } else {
                        theme.muted
                    }),
                ),
                Span::styled(
                    source.name.clone(),
                    style(if active {
                        theme.foreground
                    } else {
                        theme.muted
                    }),
                ),
            ]))
            .style(Style::default().bg(if active {
                theme.selection
            } else {
                theme.panel
            })),
            row,
        );
        app.hits.push((row, Action::Source(index)));
    }
    if source_count == 0 {
        let empty_area = Rect::new(
            inner.x,
            inner.y,
            inner.width,
            inner
                .height
                .saturating_sub(u16::from(app.notebook_selected)),
        );
        // Unlike ASCII '/', this full-cell diagonal meets the next cell's corner,
        // so adjacent rows form continuous diagonal strokes, as in Bagels.
        let hatch = "╱".repeat(empty_area.width as usize);
        for row in empty_area.y..empty_area.bottom() {
            frame.render_widget(
                Paragraph::new(hatch.as_str()).style(style(theme.border).bg(theme.panel)),
                Rect::new(empty_area.x, row, empty_area.width, 1),
            );
        }
        empty_state(
            frame,
            empty_area,
            if app.notebook_selected {
                "No sources yet"
            } else {
                "Choose a notebook first"
            },
            "",
            theme,
        );
    }
    if inner.height > 1 && app.notebook_selected {
        frame.render_widget(
            Paragraph::new(format!(" {} a add · x select", app.leader_label()))
                .style(status_style()),
            Rect::new(inner.x, inner.bottom() - 1, inner.width, 1),
        );
    }
}
