//! Widgets rendering.
use super::{
    Action, App, Block, BorderType, Borders, Color, Focus, Frame, Line, Modifier, Paragraph, Rect,
    Span, Style, Theme,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

// Status surfaces stay achromatic, independently of the selected accent palette.
pub(super) const STATUS_BG: Color = Color::Rgb(32, 32, 32);
pub(super) const STATUS_FG: Color = Color::Rgb(184, 184, 184);

pub(super) fn style(color: Color) -> Style {
    Style::default().fg(color)
}
pub(super) fn bold(color: Color) -> Style {
    style(color).add_modifier(Modifier::BOLD)
}
pub(super) fn block(title: String, active: bool, theme: Theme) -> Block<'static> {
    Block::default()
        .title(Line::from(vec![
            Span::styled(if active { " * " } else { "   " }, style(theme.accent)),
            Span::styled(
                title,
                if active {
                    bold(theme.foreground)
                } else {
                    style(theme.muted)
                },
            ),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(style(if active { theme.accent } else { theme.border }))
        .style(Style::default().bg(theme.panel).fg(theme.foreground))
}
pub(super) fn panel(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    title: &str,
    focus: Focus,
) -> Rect {
    let widget = block(title.into(), app.focus == focus, app.theme);
    let inner = widget.inner(area);
    frame.render_widget(widget, area);
    app.hits.push((area, Action::Focus(focus)));
    inner
}
pub(super) fn inset(area: Rect, x: u16, y: u16) -> Rect {
    Rect::new(
        area.x.saturating_add(x),
        area.y.saturating_add(y),
        area.width.saturating_sub(x * 2),
        area.height.saturating_sub(y * 2),
    )
}
pub(super) fn wrapped(text: &str, width: u16, color: Color) -> Vec<Line<'static>> {
    let mut lines = vec![];
    for line in text.lines() {
        if line.is_empty() {
            lines.push(Line::raw(""));
        } else {
            for part in textwrap::wrap(line, usize::from(width.max(1))) {
                lines.push(Line::styled(part.into_owned(), style(color)));
            }
        }
    }
    lines
}
pub(super) fn status_style() -> Style {
    style(STATUS_FG).bg(STATUS_BG)
}

pub(super) fn empty_state(frame: &mut Frame, area: Rect, label: &str, hint: &str, theme: Theme) {
    if area.is_empty() {
        return;
    }
    let hint_lines = wrapped(hint, area.width.saturating_sub(4).max(1), theme.muted);
    let height = 1 + if hint.is_empty() {
        0
    } else {
        hint_lines.len() + 1
    };
    let y = area.y + area.height.saturating_sub(height as u16) / 2;
    let label = Line::raw(fit_status(label, area.width.saturating_sub(2) as usize));
    for (offset, line) in std::iter::once(label)
        .chain(if hint.is_empty() {
            vec![]
        } else {
            std::iter::once(Line::raw("")).chain(hint_lines).collect()
        })
        .enumerate()
    {
        let row = y.saturating_add(offset as u16);
        if row >= area.bottom() {
            break;
        }
        let text = format!(" {line} ");
        let width = (text.width() as u16).min(area.width);
        frame.render_widget(
            Paragraph::new(text).style(style(theme.muted).bg(theme.panel)),
            Rect::new(area.x + (area.width - width) / 2, row, width, 1),
        );
    }
}

pub(super) fn body(
    frame: &mut Frame,
    app: &mut App,
    area: Rect,
    lines: Vec<Line<'static>>,
    bottom: bool,
) {
    app.scroll_max = lines.len().saturating_sub(area.height as usize);
    let offset = app
        .scroll
        .unwrap_or(if bottom { app.scroll_max } else { 0 })
        .min(app.scroll_max);
    frame.render_widget(
        Paragraph::new(lines.into_iter().skip(offset).collect::<Vec<_>>()),
        area,
    );
}

pub(super) fn fit_status(text: &str, width: usize) -> String {
    let text = text.replace(['\n', '\r', '\t'], " ");
    if text.width() <= width {
        return text;
    }
    if width == 0 {
        return String::new();
    }
    let mut result = String::new();
    let mut used = 0;
    for c in text.chars() {
        let size = c.width().unwrap_or(0);
        if used + size > width - 1 {
            break;
        }
        used += size;
        result.push(c);
    }
    result.push('…');
    result
}
