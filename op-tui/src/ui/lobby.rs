use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::LobbyState;
use crate::i18n;

pub fn draw_lobby_in(f: &mut Frame, lobby: &LobbyState, area: Rect) {
    let block = Block::default()
        .title(i18n::t("lobby-title"))
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Server URL
            Constraint::Length(3), // User ID
            Constraint::Length(2), // Connect hint
            Constraint::Length(2), // Error
            Constraint::Min(0),    // Spacer
            Constraint::Length(1), // Shortcut hints
        ])
        .split(inner);

    // Server URL field
    draw_input_field(
        f,
        chunks[0],
        &i18n::t("lobby-server-label"),
        &lobby.server_url,
        lobby.focus == 0,
    );

    // User ID / local identity field
    let user_label = if lobby.selected_identity.is_some() {
        i18n::t("lobby-user-selected-label")
    } else {
        i18n::t("lobby-user-label")
    };
    let user_value = if let Some(ref profile) = lobby.selected_identity {
        format!(
            "{} ({})",
            profile.nickname,
            truncate_user_id(&profile.user_id)
        )
    } else {
        lobby.user_id.clone()
    };
    draw_input_field(
        f,
        chunks[1],
        &user_label,
        &user_value,
        lobby.focus == 1 && lobby.selected_identity.is_none(),
    );

    // Connect hint
    let hint = Paragraph::new(i18n::t("lobby-connect"))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(hint, chunks[2]);

    // Error message
    if let Some(ref err) = lobby.error_message {
        let err_text = Paragraph::new(err.as_str())
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Red));
        f.render_widget(err_text, chunks[3]);
    }

    // Shortcut hints at bottom
    let hints = Paragraph::new(i18n::t("lobby-hint"))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(hints, chunks[5]);
}

fn draw_input_field(f: &mut Frame, area: Rect, label: &str, value: &str, focused: bool) {
    let style = if focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let border_style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let line = Line::from(vec![
        Span::styled(label, Style::default().fg(Color::Cyan)),
        Span::raw(" "),
        Span::styled(value, style),
        if focused {
            Span::styled(
                "_",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::SLOW_BLINK),
            )
        } else {
            Span::raw("")
        },
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);
    let para = Paragraph::new(line).block(block);
    f.render_widget(para, area);
}

fn truncate_user_id(user_id: &str) -> String {
    let max_len = 16;
    if user_id.len() <= max_len {
        return user_id.to_string();
    }
    let start = &user_id[..8.min(user_id.len())];
    let end = &user_id[user_id.len().saturating_sub(6)..];
    format!("{}..{}", start, end)
}
