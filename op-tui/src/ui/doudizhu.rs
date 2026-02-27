use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use fluent::fluent_args;
use openplay_doudizhu::{DouDizhuGame, PlayerState, Role, Stage};
use openplay_poker::Card;

use crate::app::GameState;
use crate::i18n;

use openplay_basic;

/// Main game rendering entry point (into a specific area, for log panel split).
pub fn draw_game_in(f: &mut Frame, gs: &GameState, area: Rect) {
    // Main layout: top opponent, middle (left opp + center + right opp), bottom hand, status bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Top opponent
            Constraint::Min(8),    // Middle area
            Constraint::Length(5), // My hand
            Constraint::Length(3), // Status bar
        ])
        .split(area);

    let Some(ref game) = gs.game else {
        // No game state yet, show waiting
        draw_waiting(f, gs, area);
        return;
    };

    let my_idx = gs.my_index.unwrap_or(0);

    // Determine opponent indices (relative to my position)
    let left_idx = (my_idx + 1) % 3;
    let right_idx = (my_idx + 2) % 3;

    // Top: right opponent (across from us)
    draw_opponent(
        f,
        main_chunks[0],
        &game.players[right_idx],
        game,
        Alignment::Center,
    );

    // Middle: left opponent + center play area + right opponent
    let mid_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // Left opponent
            Constraint::Percentage(60), // Center
            Constraint::Percentage(20), // Right opponent (info only, main display is top)
        ])
        .split(main_chunks[1]);

    draw_opponent_side(f, mid_chunks[0], &game.players[left_idx], game);
    draw_center(f, mid_chunks[1], game, gs);
    draw_opponent_side(f, mid_chunks[2], &game.players[right_idx], game);

    // Bottom: my hand
    draw_my_hand(f, main_chunks[2], gs);

    // Status bar
    draw_status_bar(f, main_chunks[3], game, gs);
}

fn draw_waiting(f: &mut Frame, gs: &GameState, area: Rect) {
    let block = Block::default()
        .title(i18n::t("app-title"))
        .borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Room title + info
            Constraint::Length(1), // Separator
            Constraint::Length(7), // 3 seats
            Constraint::Length(1), // Observers
            Constraint::Min(3),    // Messages/event log
            Constraint::Length(1), // Mode indicator
            Constraint::Length(1), // Shortcut hints
        ])
        .split(inner);

    // --- Room title & info ---
    let room_title = if let Some(ref room) = gs.room {
        let player_count = room.state.players.len();
        let args = fluent_args!["count" => player_count as i64];
        vec![
            Line::from(Span::styled(
                &room.info.title,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                i18n::t_args("room-players", &args),
                Style::default().fg(Color::White),
            )),
        ]
    } else {
        vec![Line::from(Span::styled(
            i18n::t("room-waiting"),
            Style::default().fg(Color::Yellow),
        ))]
    };
    f.render_widget(
        Paragraph::new(room_title).alignment(Alignment::Center),
        chunks[0],
    );

    // --- 3 Seats horizontal layout ---
    let seat_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(chunks[2]);

    for seat_num in 0..3u8 {
        let pos_key = seat_num.to_string();
        let pos = openplay_basic::room::RoomPlayerPosition::from(pos_key.as_str());
        let seat_area = seat_cols[seat_num as usize];

        let args = fluent_args!["num" => (seat_num + 1) as i64];
        let seat_label = i18n::t_args("room-seat", &args);

        if let Some(ref room) = gs.room {
            if let Some(player_state) = room.state.players.get(&pos) {
                // Seat occupied
                let name = &player_state.player.nickname;
                let is_bot = player_state.player.is_bot;
                let is_ready = player_state.id_ready;

                let ready_text = if is_ready {
                    i18n::t("room-seat-ready")
                } else {
                    i18n::t("room-seat-not-ready")
                };
                let ready_color = if is_ready { Color::Green } else { Color::Red };

                let mut lines = vec![
                    Line::from(Span::styled(
                        name.clone(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled(ready_text, Style::default().fg(ready_color))),
                ];
                if is_bot {
                    lines.push(Line::from(Span::styled(
                        i18n::t("room-seat-bot"),
                        Style::default().fg(Color::DarkGray),
                    )));
                }

                let border_color = if is_ready {
                    Color::Green
                } else {
                    Color::Yellow
                };
                let seat_block = Block::default()
                    .title(seat_label)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color));
                let para = Paragraph::new(lines)
                    .alignment(Alignment::Center)
                    .block(seat_block);
                f.render_widget(para, seat_area);
            } else {
                // Seat empty
                let seat_block = Block::default()
                    .title(seat_label)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray));
                let para = Paragraph::new(Line::from(Span::styled(
                    i18n::t("room-seat-empty"),
                    Style::default().fg(Color::DarkGray),
                )))
                .alignment(Alignment::Center)
                .block(seat_block);
                f.render_widget(para, seat_area);
            }
        } else {
            // No room info yet
            let seat_block = Block::default()
                .title(seat_label)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));
            let para = Paragraph::new(Line::from(Span::styled(
                "...",
                Style::default().fg(Color::DarkGray),
            )))
            .alignment(Alignment::Center)
            .block(seat_block);
            f.render_widget(para, seat_area);
        }
    }

    // --- Observers ---
    let observer_count = gs
        .room
        .as_ref()
        .map(|r| r.state.observers.len())
        .unwrap_or(0);
    let obs_args = fluent_args!["count" => observer_count as i64];
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            i18n::t_args("room-observers", &obs_args),
            Style::default().fg(Color::DarkGray),
        )))
        .alignment(Alignment::Center),
        chunks[3],
    );

    // --- Messages/event log ---
    let msg_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray));
    let msg_inner = msg_block.inner(chunks[4]);
    f.render_widget(msg_block, chunks[4]);

    let visible_lines = msg_inner.height as usize;
    let msg_lines: Vec<Line> = gs
        .messages
        .iter()
        .rev()
        .take(visible_lines)
        .rev()
        .map(|m| {
            Line::from(Span::styled(
                m.as_str(),
                Style::default().fg(Color::DarkGray),
            ))
        })
        .collect();
    f.render_widget(Paragraph::new(msg_lines), msg_inner);

    // --- Mode indicator (add-bot / kick mode) ---
    let mode_line = if gs.add_bot_mode {
        Line::from(Span::styled(
            ">> Press 1-3 to choose seat for bot (Esc: cancel) <<",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))
    } else if gs.kick_mode {
        Line::from(Span::styled(
            ">> Press 1-3 to choose seat to kick (Esc: cancel) <<",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ))
    } else {
        Line::from("")
    };
    f.render_widget(
        Paragraph::new(mode_line).alignment(Alignment::Center),
        chunks[5],
    );

    // --- Shortcut hints at bottom ---
    let hint_style = Style::default().fg(Color::DarkGray);
    let sep = Span::raw(" | ");
    let hints = Line::from(vec![
        Span::styled(i18n::t("room-sit-hint"), hint_style),
        sep.clone(),
        Span::styled(i18n::t("room-add-bot-hint"), hint_style),
        sep.clone(),
        Span::styled(i18n::t("room-kick-hint"), hint_style),
        sep.clone(),
        Span::styled(i18n::t("room-ready-hint"), hint_style),
        sep.clone(),
        Span::styled(i18n::t("room-start-hint"), hint_style),
        sep.clone(),
        Span::styled(i18n::t("game-disconnect-hint"), hint_style),
        sep.clone(),
        Span::styled(i18n::t("quit-hint"), hint_style),
    ]);
    f.render_widget(
        Paragraph::new(hints).alignment(Alignment::Center),
        chunks[6],
    );
}

fn draw_opponent(
    f: &mut Frame,
    area: Rect,
    player: &PlayerState,
    game: &DouDizhuGame,
    alignment: Alignment,
) {
    let role_str = role_string(&player.role);
    let hand_count = player.hand.len();
    let args = fluent_args!["count" => hand_count as i64];
    let remaining = i18n::t_args("cards-remaining", &args);

    let name = &player.player.nickname;
    let is_current = game.current_turn == player_index(game, player);
    let turn_indicator = if is_current { " <<" } else { "" };

    let title = format!("{} ({}){}", name, role_str, turn_indicator);
    let style = if is_current {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let block = Block::default()
        .title(title)
        .title_style(style)
        .borders(Borders::ALL)
        .border_style(if is_current {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let para = Paragraph::new(remaining).alignment(alignment).block(block);
    f.render_widget(para, area);
}

fn draw_opponent_side(f: &mut Frame, area: Rect, player: &PlayerState, _game: &DouDizhuGame) {
    let role_str = role_string(&player.role);
    let hand_count = player.hand.len();
    let args = fluent_args!["count" => hand_count as i64];
    let remaining = i18n::t_args("cards-remaining", &args);
    let name = &player.player.nickname;

    let block = Block::default()
        .title(format!("{} ({})", name, role_str))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let para = Paragraph::new(remaining)
        .alignment(Alignment::Center)
        .block(block);
    f.render_widget(para, area);
}

fn draw_center(f: &mut Frame, area: Rect, game: &DouDizhuGame, _gs: &GameState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Stage info
            Constraint::Min(3),    // Last play
            Constraint::Length(2), // Hole cards
        ])
        .split(inner);

    // Stage
    let stage_str = stage_string(&game.stage);
    let stage_line = Line::from(vec![Span::styled(
        stage_str,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]);
    f.render_widget(
        Paragraph::new(stage_line).alignment(Alignment::Center),
        chunks[0],
    );

    // Last play
    if let Some(ref last) = game.last_play {
        let player_name = game
            .players
            .get(last.player_idx)
            .map(|p| p.player.nickname.as_str())
            .unwrap_or("?");
        let cards_str = cards_to_string(&last.cards);
        let lines = vec![
            Line::from(Span::styled(player_name, Style::default().fg(Color::Green))),
            Line::from(Span::styled(
                cards_str,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
        ];
        let para = Paragraph::new(lines).alignment(Alignment::Center);
        f.render_widget(para, chunks[1]);
    } else if matches!(game.stage, Stage::Playing) {
        let para = Paragraph::new("---")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(para, chunks[1]);
    }

    // Hole cards (visible in Playing/Finished stages)
    if matches!(game.stage, Stage::Playing | Stage::Finished) && !game.hole_cards.is_empty() {
        let hole_str = cards_to_string(&game.hole_cards);
        let line = Line::from(vec![
            Span::styled(
                format!("{}: ", i18n::t("hole-cards")),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(hole_str, Style::default().fg(Color::Magenta)),
        ]);
        f.render_widget(Paragraph::new(line).alignment(Alignment::Center), chunks[2]);
    }
}

fn draw_my_hand(f: &mut Frame, area: Rect, gs: &GameState) {
    let hand = gs.my_hand();
    let my_role = gs
        .game
        .as_ref()
        .and_then(|g| gs.my_index.map(|i| &g.players[i].role))
        .cloned()
        .unwrap_or(Role::Undecided);

    let is_my_turn = gs.is_my_turn();
    let border_color = if is_my_turn {
        Color::Green
    } else {
        Color::White
    };

    let role_str = role_string(&my_role);
    let block = Block::default()
        .title(format!("You ({})", role_str))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if hand.is_empty() {
        let para = Paragraph::new("No cards")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(para, inner);
        return;
    }

    // Render each card as a span
    let mut spans: Vec<Span> = Vec::new();
    for (i, card) in hand.iter().enumerate() {
        let is_selected = gs.selected.contains(&i);
        let is_cursor = gs.cursor == i;

        let card_str = card_to_short_string(card);

        let mut style = card_color(card);
        if is_selected {
            style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
        }
        if is_cursor {
            style = style.add_modifier(Modifier::REVERSED);
        }

        spans.push(Span::styled(format!(" {} ", card_str), style));
    }

    let line = Line::from(spans);
    let para = Paragraph::new(line).alignment(Alignment::Center);
    f.render_widget(para, inner);
}

fn draw_status_bar(f: &mut Frame, area: Rect, game: &DouDizhuGame, gs: &GameState) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut parts: Vec<Span> = Vec::new();

    // Base score + multiplier
    let score_args = fluent_args!["score" => game.base_score as i64];
    parts.push(Span::styled(
        i18n::t_args("base-score", &score_args),
        Style::default().fg(Color::Cyan),
    ));
    parts.push(Span::raw(" | "));

    let mult_args = fluent_args!["mult" => game.multiplier as i64];
    parts.push(Span::styled(
        i18n::t_args("multiplier", &mult_args),
        Style::default().fg(Color::Cyan),
    ));
    parts.push(Span::raw(" | "));

    // Turn/prompt
    if gs.is_my_turn() {
        let prompt = match game.stage {
            Stage::Bidding => {
                if gs.bid_mode {
                    "0-3?".to_string()
                } else {
                    i18n::t("bid-prompt")
                }
            }
            Stage::Playing => {
                if game.last_play.is_none() || game.consecutive_passes >= 2 {
                    i18n::t("free-play-prompt")
                } else {
                    i18n::t("play-prompt")
                }
            }
            _ => i18n::t("your-turn"),
        };
        parts.push(Span::styled(
            prompt,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        let current_name = game
            .players
            .get(game.current_turn)
            .map(|p| p.player.nickname.as_str())
            .unwrap_or("?");
        let args = fluent_args!["name" => current_name.to_string()];
        parts.push(Span::styled(
            i18n::t_args("not-your-turn", &args),
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Winner
    if matches!(game.stage, Stage::Finished) {
        parts.push(Span::raw(" | "));
        let winner_msg = if game.winner == game.landlord_idx {
            i18n::t("winner-landlord")
        } else {
            i18n::t("winner-peasant")
        };
        parts.push(Span::styled(
            winner_msg,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }

    parts.push(Span::raw(" | "));
    parts.push(Span::styled(
        i18n::t("game-disconnect-hint"),
        Style::default().fg(Color::DarkGray),
    ));
    parts.push(Span::raw(" | "));
    parts.push(Span::styled(
        i18n::t("quit-hint"),
        Style::default().fg(Color::DarkGray),
    ));

    let line = Line::from(parts);
    f.render_widget(Paragraph::new(line), inner);
}

// ---- Helpers ----

fn stage_string(stage: &Stage) -> String {
    match stage {
        Stage::Setup => i18n::t("stage-waiting"),
        Stage::Bidding => i18n::t("stage-bidding"),
        Stage::Playing => i18n::t("stage-playing"),
        Stage::Finished => i18n::t("stage-finished"),
    }
}

fn role_string(role: &Role) -> String {
    match role {
        Role::Landlord => i18n::t("role-landlord"),
        Role::Peasant => i18n::t("role-peasant"),
        Role::Undecided => i18n::t("role-undecided"),
    }
}

fn player_index(game: &DouDizhuGame, player: &PlayerState) -> usize {
    game.players
        .iter()
        .position(|p| p.player.id == player.player.id)
        .unwrap_or(0)
}

fn card_to_short_string(card: &Card) -> String {
    match card {
        Card::NaturalCard(nc) => {
            let suit = match nc.suit {
                openplay_poker::Suit::Spades => "\u{2660}",
                openplay_poker::Suit::Hearts => "\u{2665}",
                openplay_poker::Suit::Diamonds => "\u{2666}",
                openplay_poker::Suit::Clubs => "\u{2663}",
            };
            let rank = match nc.rank {
                openplay_poker::Rank::Two => "2",
                openplay_poker::Rank::Three => "3",
                openplay_poker::Rank::Four => "4",
                openplay_poker::Rank::Five => "5",
                openplay_poker::Rank::Six => "6",
                openplay_poker::Rank::Seven => "7",
                openplay_poker::Rank::Eight => "8",
                openplay_poker::Rank::Nine => "9",
                openplay_poker::Rank::Ten => "10",
                openplay_poker::Rank::Jack => "J",
                openplay_poker::Rank::Queen => "Q",
                openplay_poker::Rank::King => "K",
                openplay_poker::Rank::Ace => "A",
            };
            format!("{}{}", suit, rank)
        }
        Card::RedJoker => "RJ".to_string(),
        Card::BlackJoker => "BJ".to_string(),
        Card::WildCard => "WC".to_string(),
    }
}

fn card_color(card: &Card) -> Style {
    match card {
        Card::NaturalCard(nc) => match nc.suit {
            openplay_poker::Suit::Hearts | openplay_poker::Suit::Diamonds => {
                Style::default().fg(Color::Red)
            }
            openplay_poker::Suit::Spades | openplay_poker::Suit::Clubs => {
                Style::default().fg(Color::White)
            }
        },
        Card::RedJoker => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        Card::BlackJoker => Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
        Card::WildCard => Style::default().fg(Color::Magenta),
    }
}

fn cards_to_string(cards: &[Card]) -> String {
    cards
        .iter()
        .map(card_to_short_string)
        .collect::<Vec<_>>()
        .join(" ")
}
