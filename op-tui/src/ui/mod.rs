pub mod doudizhu;
pub mod lobby;
pub mod log_panel;
pub mod user_manager;

use fluent::FluentArgs;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, LogMode, ReconnectingState, Screen};
use crate::i18n;

/// Main draw function: dispatches to the appropriate screen renderer.
/// Handles log panel overlay/split based on `app.log_mode`.
pub fn draw(f: &mut Frame, app: &App) {
    // Fullscreen log mode: takes over the entire screen
    if app.log_mode == LogMode::Fullscreen {
        log_panel::draw_log_fullscreen(f, &app.log_buffer, app.log_scroll);
        return;
    }

    // Determine main area vs log panel area
    let (main_area, log_area) = if app.log_mode == LogMode::Panel {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(6),         // Main content (at least 6 rows)
                Constraint::Percentage(33), // Log panel (~1/3 height)
            ])
            .split(f.area());
        (chunks[0], Some(chunks[1]))
    } else {
        (f.area(), None)
    };

    // Draw main content into main_area
    match &app.screen {
        Screen::Lobby(lobby) => lobby::draw_lobby_in(f, lobby, main_area),
        Screen::Connecting => draw_connecting(f, main_area),
        Screen::Game(gs) => doudizhu::draw_game_in(f, gs, main_area),
        Screen::Reconnecting(rs) => draw_reconnecting(f, rs, main_area),
        Screen::UserManager(um) => user_manager::draw_user_manager_in(f, um, main_area),
    }

    // Draw log panel if visible
    if let Some(area) = log_area {
        log_panel::draw_log_panel(f, area, &app.log_buffer);
    }
}

fn draw_connecting(f: &mut Frame, area: ratatui::layout::Rect) {
    let block = Block::default()
        .title(i18n::t("app-title"))
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Spacer top
            Constraint::Length(1), // Connecting text
            Constraint::Length(1), // Hint
            Constraint::Min(0),    // Spacer bottom
        ])
        .split(inner);

    let text = Paragraph::new(i18n::t("connecting")).alignment(Alignment::Center);
    f.render_widget(text, chunks[1]);

    let hint = Paragraph::new(i18n::t("connecting-hint"))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(hint, chunks[2]);
}

fn draw_reconnecting(f: &mut Frame, rs: &ReconnectingState, area: ratatui::layout::Rect) {
    // Draw the last known game state in the background (dimmed effect via block title)
    doudizhu::draw_game_in(f, &rs.game_state, area);

    // Calculate overlay area (centered box)
    let overlay_height = if rs.last_error.is_some() { 5 } else { 4 };
    let vert_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(overlay_height),
            Constraint::Min(0),
        ])
        .split(area);
    let horiz_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(50),
            Constraint::Min(0),
        ])
        .split(vert_chunks[1]);
    let overlay_area = horiz_chunks[1];

    // Draw overlay box
    let block = Block::default()
        .title(i18n::t("disconnected"))
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Yellow).bg(Color::Black));
    let inner = block.inner(overlay_area);
    f.render_widget(block, overlay_area);

    // Content lines
    let mut constraints = vec![
        Constraint::Length(1), // reconnecting text
    ];
    if rs.last_error.is_some() {
        constraints.push(Constraint::Length(1)); // error text
    }
    constraints.push(Constraint::Length(1)); // hint

    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let mut idx = 0;

    // "Reconnecting... attempt N/M"
    let mut args = FluentArgs::new();
    args.set("attempt", rs.attempts as i64 + 1);
    args.set("max", rs.max_attempts as i64);
    let reconnect_text = Paragraph::new(i18n::t_args("reconnecting", &args))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(reconnect_text, content_chunks[idx]);
    idx += 1;

    // Last error (if any)
    if let Some(ref err) = rs.last_error {
        let mut err_args = FluentArgs::new();
        err_args.set("error", err.clone());
        let error_text = Paragraph::new(i18n::t_args("reconnecting-error", &err_args))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Red));
        f.render_widget(error_text, content_chunks[idx]);
        idx += 1;
    }

    // Hint
    let hint = Paragraph::new(i18n::t("reconnecting-hint"))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(hint, content_chunks[idx]);
}
