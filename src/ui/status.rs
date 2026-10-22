//! Status rendering.
use super::{
    App, Block, Constraint, Frame, Layout, Modal, Paragraph, Rect, STATUS_BG, STATUS_FG, Tab, bold,
    fit_status, status_style,
};
use unicode_width::UnicodeWidthStr;

pub(super) fn status_bar(frame: &mut Frame, app: &App, area: Rect) {
    if area.is_empty() {
        return;
    }
    let mode = match &app.modal {
        Some(Modal::SummaryPlan { .. }) => "PLAN",
        Some(Modal::AiUsage(_)) => "USAGE",
        Some(Modal::Leader) => "LEADER",
        Some(Modal::Ex(_)) => "COMMAND",
        Some(Modal::SelectNotebook(_)) => "SELECT",
        Some(Modal::DeleteNotebook { .. }) => "CONFIRM",
        Some(Modal::Help) => "HELP",
        Some(Modal::Reader) => "READ",
        Some(_) => "INPUT",
        None if app.editing => "INSERT",
        None if app.tab == Tab::Notes
            && !app.notes.preview
            && app.notes.mode == crate::notes::Mode::Select =>
        {
            "SELECT"
        }
        None if !app.notebook_selected => "START",
        None => "NORMAL",
    };
    let engine = if app.pending_import.is_some() {
        "IMPORTING"
    } else if app.pending.is_some() {
        "THINKING"
    } else if app.provider.is_some() {
        ""
    } else {
        "LOCAL"
    };
    let mut right = format!("{engine} ");
    if app.notebook_selected && area.width >= 95 {
        let prefix = if engine.is_empty() {
            String::new()
        } else {
            format!("{engine}  ·  ")
        };
        right = format!(
            "{prefix}{} flashcards ",
            app.library.book().flashcard_count()
        );
    }
    let message = match &app.modal {
        _ if app.status.starts_with("SAVE FAILED:") => app.status.clone(),
        Some(Modal::Ex(line)) => format!(":{line}"),
        Some(_) => String::new(),
        None if app.editing && app.tab == Tab::Chat => "Writing a question".into(),
        None if app.editing => "Editing notes".into(),
        _ => app.status.clone(),
    };
    frame.render_widget(Block::default().style(status_style()), area);
    let badge_width = (mode.len() as u16 + 2).min(area.width);
    let right_width = (right.width() as u16).min(area.width.saturating_sub(badge_width + 4));
    let sections = Layout::horizontal([
        Constraint::Length(badge_width),
        Constraint::Min(0),
        Constraint::Length(right_width),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(format!(" {mode} ")).style(if app.transparent_background {
            bold(STATUS_FG)
        } else {
            bold(STATUS_BG).bg(STATUS_FG)
        }),
        sections[0],
    );
    let available = sections[1].width.saturating_sub(3) as usize;
    let visible = if matches!(app.modal, Some(Modal::Ex(_))) {
        let mut tail = message.clone();
        while tail.width() > available && !tail.is_empty() {
            tail.remove(0);
        }
        tail
    } else {
        fit_status(&message, available)
    };
    if matches!(app.modal, Some(Modal::Ex(_))) && available > 0 {
        frame.set_cursor_position((sections[1].x + 2 + visible.width() as u16, sections[1].y));
    }
    frame.render_widget(
        Paragraph::new(format!("  {visible}")).style(status_style()),
        sections[1],
    );
    frame.render_widget(
        Paragraph::new(right)
            .alignment(ratatui::layout::Alignment::Right)
            .style(status_style()),
        sections[2],
    );
}
