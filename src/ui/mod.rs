//! Terminal layout composition. Views render App state and emit typed Actions.
mod chat;
mod dialogs;
mod home;
mod sidebar;
mod status;
#[cfg(test)]
mod tests;
mod widgets;
mod workspace;

use chat::chat;
#[cfg(test)]
use chat::{citation_groups, composer_lines};
#[cfg(test)]
use dialogs::leader_column_width;
use dialogs::modal_view;
use home::startup;
use sidebar::sidebar;
use status::status_bar;
use widgets::{
    STATUS_BG, STATUS_FG, block, body, bold, empty_state, fit_status, inset, panel, status_style,
    style, wrapped,
};
use workspace::{main_area, source_lines};

use crate::{
    app::{Action, App, COMMANDS, Command, Focus, LEADER_BINDINGS, Modal, Tab},
    config::Theme,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

pub fn draw(frame: &mut Frame, app: &mut App) {
    draw_workspace(frame, app);
    if app.transparent_background {
        // Reset every surface, including popups and selection highlights, to the
        // terminal's default background while preserving theme foreground colors.
        for cell in &mut frame.buffer_mut().content {
            cell.set_bg(Color::Reset);
        }
    }
}

fn draw_workspace(frame: &mut Frame, app: &mut App) {
    let theme = app.theme;
    let area = frame.area();
    app.hits.clear();
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.background).fg(theme.foreground)),
        area,
    );
    if area.width < 58 || area.height < 18 {
        frame.render_widget(Paragraph::new("Kaiser\n\nGive your ideas a little more room.\nResize to at least 58 × 18.\n\nCtrl+C to exit").style(style(theme.accent)).wrap(Wrap { trim: false }), inset(area, 2, 1));
        if area.height > 0 {
            status_bar(
                frame,
                app,
                Rect::new(area.x, area.bottom() - 1, area.width, 1),
            );
        }
        return;
    }
    let rows = Layout::vertical([Constraint::Min(8), Constraint::Length(1)]).split(area);
    let workspace_area = inset(rows[0], 1, 0);
    let main = if app.notebook_selected && app.sidebar && !app.maximized && area.width >= 85 {
        let columns = Layout::horizontal([
            Constraint::Length(if area.width > 110 { 28 } else { 23 }),
            Constraint::Min(30),
        ])
        .spacing(1)
        .split(workspace_area);
        sidebar(frame, app, columns[0]);
        columns[1]
    } else {
        workspace_area
    };
    main_area(frame, app, main);
    notebook_directory(frame, app, main);
    status_bar(frame, app, rows[1]);
    if let Some(modal) = app.modal.clone() {
        app.hits.clear();
        modal_view(frame, app, modal);
    }
}

fn notebook_directory(frame: &mut Frame, app: &App, area: Rect) {
    if !app.notebook_selected {
        return;
    }
    let theme = app.theme;
    let budget = area.width.saturating_sub(4) / 2;
    if budget < 3 {
        return;
    }
    let label = format!(
        " {} ",
        directory_label(&app.library.book().directory, budget - 2)
    );
    // Paragraph styles cover their entire rectangle, including existing border
    // glyphs. Keep the rectangle tight to the label so the border stays intact.
    let width = label.width() as u16;
    let area = Rect::new(area.right().saturating_sub(width + 2), area.y, width, 1);
    frame.render_widget(
        Paragraph::new(label).style(style(theme.muted).bg(theme.panel)),
        area,
    );
}

fn directory_label(directory: &std::path::Path, width: u16) -> String {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from);
    let label = home
        .as_ref()
        .and_then(|home| directory.strip_prefix(home).ok())
        .map(|relative| {
            if relative.as_os_str().is_empty() {
                "~".into()
            } else {
                format!("~/{}", relative.display())
            }
        })
        .unwrap_or_else(|| directory.display().to_string());
    let mut label: String = label.chars().filter(|c| !c.is_control()).collect();
    let width = (label.width() as u16).min(width);
    if label.width() > width as usize {
        while label.width() > width.saturating_sub(1) as usize && !label.is_empty() {
            label.remove(0);
        }
        label.insert(0, '…');
    }
    label
}
