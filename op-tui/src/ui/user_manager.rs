use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{UserManagerMode, UserManagerState};
use crate::i18n;

pub fn draw_user_manager_in(f: &mut Frame, um: &UserManagerState, area: Rect) {
    let block = Block::default()
        .title(i18n::t("user-manager-title"))
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Mode / status
            Constraint::Min(6),    // List
            Constraint::Length(3), // Create input / delete confirm
            Constraint::Length(1), // Error
            Constraint::Length(1), // Hints
        ])
        .split(inner);

    let status = match um.mode {
        UserManagerMode::Browse => i18n::t("user-manager-mode-browse"),
        UserManagerMode::Create => i18n::t("user-manager-mode-create"),
        UserManagerMode::DeleteConfirm => i18n::t("user-manager-mode-delete"),
    };
    let status_para = Paragraph::new(status)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(status_para, chunks[0]);

    let items: Vec<ListItem> = if um.profiles.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            i18n::t("user-manager-empty"),
            Style::default().fg(Color::DarkGray),
        )]))]
    } else {
        um.profiles
            .iter()
            .map(|profile| {
                let label = format!(
                    "{}  {}",
                    profile.nickname,
                    truncate_user_id(&profile.user_id)
                );
                ListItem::new(Line::from(vec![Span::raw(label)]))
            })
            .collect()
    };
    let mut list_state = list_state(um);
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    f.render_stateful_widget(list, chunks[1], &mut list_state);

    match um.mode {
        UserManagerMode::Create => {
            let input = Paragraph::new(Line::from(vec![
                Span::styled(
                    i18n::t("user-manager-nickname"),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(" "),
                Span::styled(&um.input, Style::default().fg(Color::White)),
                Span::styled(
                    "_",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::SLOW_BLINK),
                ),
            ]))
            .block(Block::default().borders(Borders::ALL));
            f.render_widget(input, chunks[2]);
        }
        UserManagerMode::DeleteConfirm => {
            let confirm = Paragraph::new(i18n::t("user-manager-delete-confirm"))
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(confirm, chunks[2]);
        }
        UserManagerMode::Browse => {
            let hint = Paragraph::new(i18n::t("user-manager-select-hint"))
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(hint, chunks[2]);
        }
    }

    if let Some(ref err) = um.error_message {
        let err_text = Paragraph::new(err.as_str())
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Red));
        f.render_widget(err_text, chunks[3]);
    }

    let hints = Paragraph::new(i18n::t("user-manager-hint"))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(hints, chunks[4]);
}

fn list_state(um: &UserManagerState) -> ratatui::widgets::ListState {
    let mut state = ratatui::widgets::ListState::default();
    if !um.profiles.is_empty() {
        state.select(Some(um.selected));
    }
    state
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
