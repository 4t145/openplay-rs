use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use tracing::Level;

use crate::i18n;
use crate::log_buffer::LogBuffer;

/// Draw the log panel at the bottom of the screen (Panel mode).
/// Returns nothing — renders directly into the given `area`.
pub fn draw_log_panel(f: &mut Frame, area: Rect, log_buffer: &LogBuffer) {
    let block = Block::default()
        .title(format!(" {} ", i18n::t("log-title")))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible_lines = inner.height as usize;
    let entries = log_buffer.recent(visible_lines);

    if entries.is_empty() {
        let empty = Paragraph::new(i18n::t("log-empty"))
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(empty, inner);
        return;
    }

    let lines: Vec<Line> = entries.iter().map(|e| format_log_line(e)).collect();

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(paragraph, inner);
}

/// Draw the log panel in fullscreen (Fullscreen mode).
/// `scroll_offset` is lines from the bottom (0 = most recent visible).
pub fn draw_log_fullscreen(f: &mut Frame, log_buffer: &LogBuffer, scroll_offset: usize) {
    let area = f.area();

    let block = Block::default()
        .title(format!(" {} ", i18n::t("log-title")))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Reserve 1 line at the bottom for hint bar
    if inner.height < 2 {
        return;
    }
    let log_area = Rect {
        height: inner.height - 1,
        ..inner
    };
    let hint_area = Rect {
        y: inner.y + inner.height - 1,
        height: 1,
        ..inner
    };

    // Draw hint
    let hint = Paragraph::new(i18n::t("log-fullscreen-hint"))
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(hint, hint_area);

    let all_entries = log_buffer.all();
    if all_entries.is_empty() {
        let empty = Paragraph::new(i18n::t("log-empty"))
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(empty, log_area);
        return;
    }

    let lines: Vec<Line> = all_entries.iter().map(|e| format_log_line(e)).collect();
    let total = lines.len();
    let visible = log_area.height as usize;

    // Clamp scroll so we don't scroll past the top
    let max_scroll = total.saturating_sub(visible);
    let clamped_scroll = scroll_offset.min(max_scroll);

    // scroll parameter is (rows_from_top, cols_from_left)
    let scroll_from_top = total.saturating_sub(visible).saturating_sub(clamped_scroll);

    let paragraph = Paragraph::new(lines)
        .scroll((scroll_from_top as u16, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, log_area);
}

/// Format a log entry as a styled Line.
fn format_log_line(entry: &crate::log_buffer::LogEntry) -> Line<'static> {
    let level_color = match entry.level {
        Level::ERROR => Color::Red,
        Level::WARN => Color::Yellow,
        Level::INFO => Color::Green,
        Level::DEBUG => Color::Blue,
        Level::TRACE => Color::DarkGray,
    };

    Line::from(vec![
        Span::styled(
            format!("{} ", entry.timestamp),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            format!("{:>5} ", entry.level),
            Style::default()
                .fg(level_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(entry.message.clone(), Style::default().fg(Color::White)),
    ])
}
