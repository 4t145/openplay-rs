use bytes::Bytes;
use openplay_basic::player::Player;
use openplay_holdem::{BettingAction, GameEvent, GameStage, HoldemGame};
use openplay_poker::fmt::Cards;

fn create_player(id: u8, nickname: &str) -> Player {
    Player {
        id: openplay_basic::player::PlayerId::from(Bytes::from(vec![id])),
        nickname: nickname.to_string(),
        avatar_url: None,
        is_bot: false,
    }
}

fn print_event(event: &GameEvent) {
    match event {
        GameEvent::GameStarted {
            players,
            dealer_position,
        } => {
            println!("🎮 游戏开始!");
            println!("   庄家位置: Player {}", dealer_position);
            println!(
                "   玩家: {:?}",
                players.iter().map(|p| &p.nickname).collect::<Vec<_>>()
            );
        }
        GameEvent::BlindsPosted {
            small_blind_player,
            small_blind_amount,
            big_blind_player,
            big_blind_amount,
        } => {
            println!("💰 盲注:");
            println!(
                "   小盲注: Player {} 下注 ${}",
                small_blind_player, small_blind_amount
            );
            println!(
                "   大盲注: Player {} 下注 ${}",
                big_blind_player, big_blind_amount
            );
        }
        GameEvent::CardsDealt { player, cards } => {
            println!("🎴 Player {} 收到手牌: {}", player, Cards(cards));
        }
        GameEvent::StageChanged { stage } => {
            println!("\n📍 阶段变更: {:?}", stage);
        }
        GameEvent::CommunityCardsRevealed { cards } => {
            println!("🃏 公共牌: {}", Cards(cards));
        }
        GameEvent::PlayerAction { player, action } => {
            println!("   Player {} 行动: {:?}", player, action);
        }
        GameEvent::PotUpdated { pot } => {
            println!("   💰 底池: ${}", pot);
        }
        GameEvent::PlayerWon { player, amount, .. } => {
            println!("🏆 Player {} 赢得 ${}", player, amount);
        }
        GameEvent::GameEnded => {
            println!("✅ 游戏结束");
        }
    }
}

fn print_game_state(game: &HoldemGame) {
    println!("\n{}", "═".repeat(60));
    println!("游戏状态:");
    println!("  阶段: {:?}", game.get_stage());
    println!("  底池: ${}", game.get_pot());
    println!(
        "  公共牌 [{}]: {}",
        game.get_community_cards().len(),
        Cards(game.get_community_cards())
    );
    println!("  当前行动玩家: Player {}", game.get_current_player());
    println!();

    for (i, player) in game.players.iter().enumerate() {
        let status = if !player.is_active {
            "已弃牌"
        } else if player.is_all_in {
            "全下"
        } else {
            "活跃"
        };

        let empty_hand = vec![];
        let hand_display = if player.is_active {
            &player.hand
        } else {
            &empty_hand
        };

        println!(
            "  Player {} ({}) - {} | 筹码: ${} | 当前下注: ${} | 手牌: {}",
            i, player.player.nickname, status, player.chips, player.current_bet, Cards(hand_display)
        );
    }
    println!("{}", "═".repeat(60));
}

fn print_valid_actions(game: &HoldemGame) {
    let current = game.get_current_player();
    let actions = game.get_valid_actions(current);

    if !actions.is_empty() {
        println!("\n可执行的动作:");
        for (i, action) in actions.iter().enumerate() {
            println!("  {}: {:?}", i + 1, action);
        }
    }
}

fn main() {
    println!("🎰 德州扑克 - 完整游戏演示\n");

    // 创建3个玩家
    let players = vec![
        create_player(1, "Alice"),
        create_player(2, "Bob"),
        create_player(3, "Charlie"),
    ];

    // 创建游戏: 起始筹码1000, 小盲注10, 大盲注20
    let mut game = HoldemGame::new(players, 1000, 10, 20).unwrap();

    // 开始游戏
    let events = game.start_round();

    println!("{}", "═".repeat(60));
    println!("游戏事件日志:");
    println!("{}", "═".repeat(60));
    for event in events {
        print_event(&event);
    }

    print_game_state(&game);

    // 自动游戏流程
    println!("\n开始自动游戏流程...\n");

    let mut round = 1;
    let max_rounds = 20; // 防止无限循环

    while game.get_stage() != GameStage::Finished && round <= max_rounds {
        let current = game.get_current_player();
        let player_state = game.get_player_state(current).unwrap();

        if !player_state.is_active || player_state.is_all_in {
            break;
        }

        println!(
            "\n轮次 {}: Player {} ({}) 行动中...",
            round, current, player_state.player.nickname
        );

        print_valid_actions(&game);

        // 简单AI策略
        let action = choose_action(&game, current);

        println!("➡️  选择: {:?}", action);

        match game.player_action(current, action) {
            Ok(events) => {
                for event in events {
                    print_event(&event);
                }

                // 如果阶段变化，显示当前状态
                if game.get_stage() != GameStage::PreFlop || round > 3 {
                    print_game_state(&game);
                }
            }
            Err(e) => {
                println!("❌ 错误: {:?}", e);
                break;
            }
        }

        round += 1;

        // 检查是否只剩一个活跃玩家
        let active_count = game.players.iter().filter(|p| p.is_active).count();
        if active_count <= 1 {
            println!("\n⚠️  只剩一个活跃玩家，游戏提前结束");
            break;
        }
    }

    let separator = "═".repeat(60);
    println!("\n{}", separator);
    println!("最终结果:");
    println!("{}", separator);

    for (i, player) in game.players.iter().enumerate() {
        let change = (player.chips as i64) - 1000;
        let sign = if change >= 0 { "+" } else { "" };
        println!(
            "  Player {} ({}) - 筹码: ${} ({}${})",
            i, player.player.nickname, player.chips, sign, change
        );
    }

    println!("{}", separator);
}

// 简单的AI决策逻辑
fn choose_action(game: &HoldemGame, player_idx: usize) -> BettingAction {
    let actions = game.get_valid_actions(player_idx);
    let player = game.get_player_state(player_idx).unwrap();
    let pot = game.get_pot();

    // 随机策略（实际游戏中应该基于牌力）
    use rand::Rng;
    let mut rng = rand::rng();
    let decision = rng.random_range(0..100);

    // 如果可以过牌，70%的概率过牌
    if actions.contains(&BettingAction::Check) && decision < 70 {
        return BettingAction::Check;
    }

    // 如果需要跟注
    if actions.contains(&BettingAction::Call) {
        let call_cost = game.current_bet - player.current_bet;
        let pot_odds = call_cost as f64 / (pot + call_cost) as f64;

        // 简单的pot odds决策
        if pot_odds < 0.3 && decision < 80 {
            return BettingAction::Call;
        } else if decision < 20 {
            return BettingAction::Call;
        }
    }

    // 尝试下注或加注
    if let Some(bet) = actions.iter().find(|a| matches!(a, BettingAction::Bet(_))) {
        if decision >= 70 && decision < 85 {
            return *bet;
        }
    }

    if let Some(raise) = actions
        .iter()
        .find(|a| matches!(a, BettingAction::Raise(_)))
    {
        if decision >= 85 && decision < 95 {
            return *raise;
        }
    }

    // 小概率All-In
    if actions.contains(&BettingAction::AllIn) && decision >= 95 {
        return BettingAction::AllIn;
    }

    // 默认：弃牌
    BettingAction::Fold
}
