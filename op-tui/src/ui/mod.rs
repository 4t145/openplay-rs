pub mod doudizhu;
pub mod lobby;
pub mod log_panel;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use crate::app::{App, LogMode, Screen};
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
    }

    // Draw log panel if visible
    if let Some(area) = log_area {
        log_panel::draw_log_panel(f, area, &app.log_buffer);
    }
}

fn draw_connecting(f: &mut Frame, area: ratatui::layout::Rect) {
    use ratatui::layout::Alignment;
    use ratatui::style::{Color, Style};
    use ratatui::widgets::{Block, Borders, Paragraph};

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
