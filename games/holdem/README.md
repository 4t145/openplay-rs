# 德州扑克 (Texas Hold'em) 游戏实现

## 概述

这是一个完整的德州扑克游戏实现，支持标准的德州扑克规则和完整的游戏流程。

## 核心组件

### 1. 游戏阶段 (GameStage)

```rust
pub enum GameStage {
    Setup,    // 准备阶段
    PreFlop,  // 翻牌前
    Flop,     // 翻牌 (3张公共牌)
    Turn,     // 转牌 (第4张公共牌)
    River,    // 河牌 (第5张公共牌)
    Showdown, // 摊牌
    Finished, // 结束
}
```

### 2. 玩家动作 (BettingAction)

```rust
pub enum BettingAction {
    Fold,       // 弃牌 - 放弃本局
    Check,      // 过牌 - 不下注但继续游戏
    Call,       // 跟注 - 跟上当前最高注
    Bet(u64),   // 下注 - 主动下注
    Raise(u64), // 加注 - 在当前注的基础上加注
    AllIn,      // 全下 - 下注所有筹码
}
```

### 3. 玩家状态 (PlayerState)

```rust
pub struct PlayerState {
    pub player: Player,      // 玩家信息
    pub chips: u64,          // 当前筹码
    pub hand: Vec<Card>,     // 手牌 (2张)
    pub current_bet: u64,    // 本轮当前下注
    pub total_bet: u64,      // 本局总下注
    pub is_active: bool,     // 是否还在游戏中
    pub is_all_in: bool,     // 是否全下
}
```

### 4. 游戏事件 (GameEvent)

所有游戏行为都会产生事件，用于日志记录和前端展示：

```rust
pub enum GameEvent {
    GameStarted { players, dealer_position },
    BlindsPosted { small_blind_player, small_blind_amount, big_blind_player, big_blind_amount },
    CardsDealt { player, cards },
    StageChanged { stage },
    CommunityCardsRevealed { cards },
    PlayerAction { player, action },
    PotUpdated { pot },
    PlayerWon { player, amount, hand_rank },
    GameEnded,
}
```

## 游戏流程

### 完整流程图

```
1. 创建游戏 (HoldemGame::new)
   ├─ 设置玩家 (2-10人)
   ├─ 设置盲注 (小盲注/大盲注)
   └─ 初始化筹码

2. 开始新一轮 (start_round)
   ├─ 洗牌
   ├─ 收取盲注
   │  ├─ 庄家+1位: 小盲注
   │  └─ 庄家+2位: 大盲注
   └─ 发2张手牌给每位玩家

3. 翻牌前 (PreFlop)
   ├─ 从大盲注+1位开始行动
   └─ 每位玩家依次行动直到下注完成

4. 翻牌 (Flop)
   ├─ 烧1张牌
   ├─ 发3张公共牌
   ├─ 从小盲注位开始新一轮下注
   └─ 每位玩家依次行动

5. 转牌 (Turn)
   ├─ 烧1张牌
   ├─ 发第4张公共牌
   └─ 新一轮下注

6. 河牌 (River)
   ├─ 烧1张牌
   ├─ 发第5张公共牌
   └─ 最后一轮下注

7. 摊牌 (Showdown)
   ├─ 剩余玩家亮牌
   ├─ 比较牌型大小
   └─ 确定赢家

8. 结算 (Settlement)
   ├─ 分配底池
   ├─ 庄家位置移动
   └─ 准备下一局
```

### 下注轮流程

每轮下注遵循以下规则：

1. **行动顺序**：按座位顺序顺时针进行
2. **下注完成条件**：
   - 所有活跃玩家的当前下注相等
   - 只剩一个活跃玩家（其他人弃牌）
3. **可执行动作检查**：
   - 当前下注=0: 可以 Check 或 Bet
   - 当前下注>0: 必须 Call、Raise 或 Fold
   - 筹码不足: 只能 All-In 或 Fold

## API 使用示例

### 创建游戏

```rust
use openplay_holdem::{HoldemGame, BettingAction};
use openplay_basic::player::Player;

let players = vec![
    create_player(1, "Alice"),
    create_player(2, "Bob"),
    create_player(3, "Charlie"),
];

// 创建游戏: 起始筹码1000, 小盲注10, 大盲注20
let mut game = HoldemGame::new(players, 1000, 10, 20)?;
```

### 开始新一轮

```rust
// 开始游戏，返回事件列表
let events = game.start_round();

for event in events {
    match event {
        GameEvent::GameStarted { .. } => println!("游戏开始"),
        GameEvent::CardsDealt { player, cards } => {
            println!("Player {} 收到手牌: {:?}", player, cards);
        }
        _ => {}
    }
}
```

### 玩家行动

```rust
// 获取当前行动玩家
let current_player = game.get_current_player();

// 检查可用动作
let valid_actions = game.get_valid_actions(current_player);
println!("可用动作: {:?}", valid_actions);

// 执行动作
let events = game.player_action(current_player, BettingAction::Call)?;

// 处理事件
for event in events {
    println!("事件: {:?}", event);
}
```

### 查询游戏状态

```rust
// 获取当前阶段
let stage = game.get_stage();

// 获取底池
let pot = game.get_pot();

// 获取公共牌
let community_cards = game.get_community_cards();

// 获取玩家状态
if let Some(player_state) = game.get_player_state(0) {
    println!("筹码: {}", player_state.chips);
    println!("手牌: {:?}", player_state.hand);
}
```

## 运行测试

### 运行单元测试

```bash
cargo test --package openplay-holdem
```

### 测试用例

- `test_game_creation` - 游戏创建
- `test_not_enough_players` - 玩家数量检查
- `test_start_round` - 游戏开始流程
- `test_full_game_flow` - 完整游戏流程
- `test_player_fold` - 玩家弃牌
- `test_invalid_action_not_turn` - 非法行动检查
- `test_betting_and_raising` - 下注和加注

### 运行交互式示例

```bash
cargo run --example interactive_game
```

这个示例展示了：
- 3个玩家的完整游戏
- 从PreFlop到Showdown的所有阶段
- 简单的AI决策逻辑
- 详细的事件日志和游戏状态输出

## 特性

### 已实现

- ✅ 完整的游戏流程 (PreFlop → Flop → Turn → River → Showdown)
- ✅ 所有标准动作 (Fold, Check, Call, Bet, Raise, All-In)
- ✅ 盲注系统
- ✅ 底池管理
- ✅ 多玩家支持 (2-10人)
- ✅ 烧牌机制
- ✅ 事件系统
- ✅ 动作验证
- ✅ 玩家状态管理

### 待实现

- ⏳ 牌型评估 (Hand Ranking)
  - Royal Flush (皇家同花顺)
  - Straight Flush (同花顺)
  - Four of a Kind (四条)
  - Full House (葫芦)
  - Flush (同花)
  - Straight (顺子)
  - Three of a Kind (三条)
  - Two Pair (两对)
  - One Pair (一对)
  - High Card (高牌)
- ⏳ 边池 (Side Pot) 处理
- ⏳ 平局处理 (Split Pot)
- ⏳ 更复杂的AI策略
- ⏳ 游戏历史记录
- ⏳ 断线重连

## 错误处理

```rust
pub enum GameError {
    NotEnoughPlayers,              // 玩家数量不足
    InvalidPlayerIndex,            // 无效的玩家索引
    InvalidAction(String),         // 无效的动作
    NotPlayerTurn,                 // 不是该玩家的回合
    InsufficientChips,             // 筹码不足
    GameNotInProgress,             // 游戏未进行
    InvalidBetAmount,              // 无效的下注金额
}
```

## 设计原则

1. **事件驱动**：所有游戏行为都产生事件，便于日志和前端同步
2. **状态不可变**：游戏状态通过方法调用改变，保证一致性
3. **类型安全**：使用强类型表达游戏概念，避免运行时错误
4. **可测试**：核心逻辑独立，易于单元测试
5. **可扩展**：预留接口用于实现高级特性

## 性能考虑

- 使用 `Vec` 而非 `HashMap` 存储玩家，因为玩家数量少
- 卡牌使用 `Copy` trait，避免不必要的克隆
- 事件系统允许按需处理，不强制同步所有状态

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可

MIT License
