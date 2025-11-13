# 快速开始指南

## 5分钟上手德州扑克游戏

### 1. 安装依赖

```bash
cd /home/atlas/Github/openplay-holdem
cargo build
```

### 2. 运行测试

```bash
# 运行所有测试
cargo test --package openplay-holdem

# 运行测试并查看输出
cargo test --package openplay-holdem -- --nocapture
```

### 3. 运行交互式示例

```bash
cargo run --example interactive_game
```

你会看到一个完整的3人德州扑克游戏演示！

### 4. 基础用法

#### 创建游戏

```rust
use openplay_holdem::{HoldemGame, BettingAction};
use openplay_basic::player::Player;
use bytes::Bytes;

// 创建玩家
fn create_player(id: u8, nickname: &str) -> Player {
    Player {
        id: openplay_basic::player::PlayerId::from(Bytes::from(vec![id])),
        nickname: nickname.to_string(),
        avatar_url: None,
        is_bot: false,
    }
}

fn main() {
    let players = vec![
        create_player(1, "Alice"),
        create_player(2, "Bob"),
    ];

    // 创建游戏: 1000筹码, 10/20盲注
    let mut game = HoldemGame::new(players, 1000, 10, 20).unwrap();
    
    // 开始游戏
    let events = game.start_round();
    println!("游戏开始! 事件数: {}", events.len());
}
```

#### 玩家行动

```rust
// 获取当前玩家
let current = game.get_current_player();
println!("轮到Player {} 行动", current);

// 查看可用动作
let actions = game.get_valid_actions(current);
println!("可用动作: {:?}", actions);

// 执行动作
match game.player_action(current, BettingAction::Call) {
    Ok(events) => {
        println!("行动成功! 产生了 {} 个事件", events.len());
    }
    Err(e) => {
        println!("行动失败: {:?}", e);
    }
}
```

#### 查询游戏状态

```rust
// 当前阶段
println!("阶段: {:?}", game.get_stage());

// 底池
println!("底池: ${}", game.get_pot());

// 公共牌
println!("公共牌: {:?}", game.get_community_cards());

// 玩家信息
if let Some(player) = game.get_player_state(0) {
    println!("Player 0:");
    println!("  筹码: ${}", player.chips);
    println!("  手牌: {:?}", player.hand);
    println!("  当前下注: ${}", player.current_bet);
    println!("  活跃: {}", player.is_active);
}
```

### 5. 完整游戏循环

```rust
fn main() {
    let players = vec![
        create_player(1, "Alice"),
        create_player(2, "Bob"),
    ];
    
    let mut game = HoldemGame::new(players, 1000, 10, 20).unwrap();
    game.start_round();
    
    // 游戏循环
    while game.get_stage() != GameStage::Finished {
        let current = game.get_current_player();
        let player = game.get_player_state(current).unwrap();
        
        if !player.is_active || player.is_all_in {
            break;
        }
        
        // 简单策略: 如果可以过牌就过牌，否则跟注
        let actions = game.get_valid_actions(current);
        let action = if actions.contains(&BettingAction::Check) {
            BettingAction::Check
        } else if actions.contains(&BettingAction::Call) {
            BettingAction::Call
        } else {
            BettingAction::Fold
        };
        
        match game.player_action(current, action) {
            Ok(events) => {
                for event in events {
                    println!("{:?}", event);
                }
            }
            Err(e) => {
                println!("错误: {:?}", e);
                break;
            }
        }
    }
    
    // 显示结果
    println!("\n游戏结束!");
    for (i, player) in game.players.iter().enumerate() {
        println!("Player {}: ${}", i, player.chips);
    }
}
```

### 6. 事件处理示例

```rust
use openplay_holdem::GameEvent;

fn handle_event(event: &GameEvent) {
    match event {
        GameEvent::GameStarted { players, dealer_position } => {
            println!("🎮 游戏开始! 庄家: Player {}", dealer_position);
        }
        GameEvent::BlindsPosted { small_blind_player, small_blind_amount, .. } => {
            println!("💰 Player {} 下小盲注 ${}", small_blind_player, small_blind_amount);
        }
        GameEvent::CardsDealt { player, cards } => {
            println!("🎴 Player {} 收到手牌 [{} cards]", player, cards.len());
        }
        GameEvent::StageChanged { stage } => {
            println!("📍 进入阶段: {:?}", stage);
        }
        GameEvent::CommunityCardsRevealed { cards } => {
            println!("🃏 公共牌: {:?}", cards);
        }
        GameEvent::PlayerAction { player, action } => {
            println!("👤 Player {} 行动: {:?}", player, action);
        }
        GameEvent::PotUpdated { pot } => {
            println!("💰 底池更新: ${}", pot);
        }
        GameEvent::PlayerWon { player, amount, .. } => {
            println!("🏆 Player {} 赢得 ${}", player, amount);
        }
        GameEvent::GameEnded => {
            println!("✅ 游戏结束");
        }
    }
}

// 使用
let events = game.start_round();
for event in events {
    handle_event(&event);
}
```

### 7. 常见问题

#### Q: 如何处理 "Not your turn" 错误？

```rust
if game.get_current_player() != player_idx {
    return Err("等待其他玩家行动");
}
```

#### Q: 如何检查玩家是否可以执行某个动作？

```rust
let actions = game.get_valid_actions(player_idx);
if !actions.contains(&BettingAction::Raise(100)) {
    println!("不能加注到100");
}
```

#### Q: 如何实现超时自动弃牌？

```rust
use std::time::{Duration, Instant};

let timeout = Duration::from_secs(30);
let start = Instant::now();

while start.elapsed() < timeout {
    // 等待玩家行动
    if let Some(action) = check_player_action() {
        game.player_action(current, action)?;
        break;
    }
}

// 超时自动弃牌
if start.elapsed() >= timeout {
    game.player_action(current, BettingAction::Fold)?;
}
```

### 8. 下一步

- 阅读 [README.md](README.md) 了解完整API
- 阅读 [DESIGN.md](DESIGN.md) 了解架构设计
- 查看 `examples/interactive_game.rs` 学习完整示例
- 运行 `cargo test` 查看所有测试用例

### 9. 调试技巧

```rust
// 打印游戏状态
println!("阶段: {:?}", game.get_stage());
println!("当前玩家: {}", game.get_current_player());
println!("底池: ${}", game.get_pot());

// 打印所有玩家状态
for (i, player) in game.players.iter().enumerate() {
    println!("Player {}: chips={}, bet={}, active={}", 
        i, player.chips, player.current_bet, player.is_active);
}

// 打印可用动作
let actions = game.get_valid_actions(game.get_current_player());
println!("可用动作: {:?}", actions);
```

### 10. 性能提示

- 游戏对象很轻量，可以为每个房间创建独立实例
- 事件可以异步处理，不会阻塞游戏逻辑
- 使用 `Clone` trait 可以安全地复制游戏状态（用于回滚或模拟）

祝你游戏愉快！🎰
