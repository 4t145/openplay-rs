---
name: game-module-dev
description: Step-by-step workflow for developing new game modules in the OpenPlay framework - covers game rules research, state machine design with INSTRUCTION.md, core implementation, unit testing, API documentation, and optional TUI client
---

# OpenPlay 游戏模块开发技能

本技能指导你为 OpenPlay 框架开发新的游戏模块。严格按以下阶段顺序执行，每个阶段完成后必须停下来等待开发者 review 确认，确认后方可进入下一阶段。

---

## 工作流程总览

```
Phase 1: 游戏规格设计        → 输出 games/<name>/INSTRUCTION.md
  ⏸️ 等待开发者 review
Phase 2: 游戏核心实现        → 输出 games/<name>/src/ 完整代码
  ⏸️ 等待开发者 review
Phase 3: 单元测试            → 输出测试代码并全部通过
  ⏸️ 等待开发者 review
Phase 4 (可选): TUI 客户端   → 输出 op-tui 中的游戏渲染模块
  ⏸️ 等待开发者 review
Phase 5 (可选): 集成接入     → 输出 op-host 中的注册和 Bot 代码
  ⏸️ 等待开发者 review
```

---

## Phase 1: 游戏规格设计

### 目标
产出一份完整的游戏规格说明书 `games/<name>/INSTRUCTION.md`，作为后续所有开发的唯一指导文件。

### 步骤

1. **需求确认** — 向开发者确认：
   - 游戏名称和标识符（APP_ID，小写英文，如 `holdem`, `mahjong`）
   - 玩家人数（固定 or 范围）
   - 是否需要 Bot 支持
   - 是否需要 TUI 客户端
   - 任何特殊规则变体

2. **规则研究** — 通过搜索或由开发者提供游戏的通行规则，确保理解完整的游戏流程。

3. **编写 INSTRUCTION.md** — 文件放在 `games/<name>/INSTRUCTION.md`，内容全部使用伪代码描述，不涉及具体编程语言代码。必须包含以下章节：

#### INSTRUCTION.md 必须包含的章节

```markdown
# <游戏名称> 游戏规格说明

## 1. 游戏规则摘要
- 游戏的基本规则描述
- 玩家人数
- 核心玩法

## 2. 状态机设计

### 2.1 阶段定义 (Stage)
- 列出所有游戏阶段
- 每个阶段的描述
- 阶段转换条件和流向图

### 2.2 服务器状态结构
- 完整的游戏状态字段定义（伪代码）
- 每个字段的用途说明
- 哪些字段需要对客户端隐藏（#[serde(skip)]）

### 2.3 玩家动作定义 (Action)
- 每个阶段允许的动作枚举
- 每个动作携带的数据
- 动作的合法性校验规则

### 2.4 动作处理流程
- 每个动作的处理伪代码
- 状态转移逻辑
- 边界条件处理

### 2.5 超时与默认动作 (default_action)
- 每个阶段的超时行为
- default_action 的 fallback 策略

### 2.6 计分规则
- 分数计算方式
- 倍数/乘数机制（如有）

### 2.7 胜负判定
- 游戏结束条件
- 赢家确定规则

## 3. 客户端 API 设计

### 3.1 视图结构
- 各角色（Position）能看到的数据
- 遮蔽规则：哪些字段对其他玩家隐藏，如何隐藏
- 观察者（Neutral）视图
- 视图数据的 JSON 结构示例

### 3.2 客户端动作格式
- 客户端发送每种动作的 JSON 结构示例
- TypedData 信封包装说明

### 3.3 交互协议
- 每个阶段的客户端-服务器交互流程
- 版本号（乐观锁）的使用说明

## 4. 测试场景清单
- 列出需要覆盖的测试场景
- 包括正常流程、边界情况、非法操作
```

### 交付物
- `games/<name>/INSTRUCTION.md`

### ⏸️ 停止并等待开发者 review
将 INSTRUCTION.md 的内容展示给开发者，讨论并修改直到开发者满意。明确告知开发者："Phase 1 完成，请 review INSTRUCTION.md，确认后我将开始 Phase 2 实现。"

---

## Phase 2: 游戏核心实现

### 目标
根据 INSTRUCTION.md 实现完整的游戏 crate。

### 步骤

1. **创建 crate 结构**：
   ```
   games/<name>/
   ├── Cargo.toml
   └── src/
       ├── lib.rs          # 游戏主逻辑：状态 struct + Game trait 实现
       └── (其他模块)       # 按需拆分（如 pattern.rs, scoring.rs 等）
   ```

2. **实现游戏状态 struct**：根据 INSTRUCTION.md 第 2.2 节

3. **实现 Action 枚举**：根据 INSTRUCTION.md 第 2.3 节

4. **实现 Game trait 的 5 个方法**：
   - `meta()` — 返回游戏元数据
   - `handle_action()` — 核心处理循环
   - `current_view()` — 当前状态快照（重连用）
   - `default_action()` — 超时 fallback 策略
   - `apply_config()` — 配置更新（可选）

5. **实现辅助方法**：
   - `start()` — 从 RoomContext 读取玩家列表，初始化游戏
   - `make_update()` — 生成各视角的 GameUpdate
   - `masked_snapshot()` — 遮蔽视图生成
   - `snapshot()` — 服务端完整快照
   - Timer 管理相关方法

6. **验证编译通过**：`cargo build -p openplay-<name>`

### 交付物
- `games/<name>/` 完整 crate 代码
- 编译无错误

### ⏸️ 停止并等待开发者 review
明确告知开发者："Phase 2 完成，游戏核心代码已实现并编译通过，请 review 代码，确认后我将开始 Phase 3 编写测试。"

---

## Phase 3: 单元测试

### 目标
编写全面的单元测试，验证游戏逻辑的正确性。

### 测试范围（参照 INSTRUCTION.md 第 4 节的测试场景清单）
- 游戏初始化正确性
- 每个阶段的合法动作处理
- 非法动作被正确拒绝（错误阶段、非当前玩家、无效数据等）
- 阶段转换逻辑
- 视图遮蔽正确性（每个玩家只能看到自己该看到的）
- 超时 / default_action 行为
- 胜负判定
- 边界情况
- 乐观锁版本检查

### 注意事项
- 测试只关注游戏自身逻辑，不涉及 RoomService 集成
- 测试中手动构造 RoomContext（含 Room 和 players）
- 使用 `cargo test -p openplay-<name>` 运行
- 所有测试必须通过

### 交付物
- 测试代码（在 `src/lib.rs` 的 `#[cfg(test)] mod tests` 中，或独立的 `tests/` 目录）
- `cargo test -p openplay-<name>` 全部通过

### ⏸️ 停止并等待开发者 review
明确告知开发者："Phase 3 完成，所有 N 个测试通过，请 review 测试覆盖范围，确认后可以进入可选阶段。"

---

## Phase 4 (可选): TUI 客户端

### 前提
开发者在 Phase 1 确认需要 TUI 客户端。

### 目标
根据 INSTRUCTION.md 的客户端 API 设计 + 已有游戏代码，实现 TUI 渲染模块。

### 步骤
1. 在 `op-tui/Cargo.toml` 添加游戏 crate 依赖
2. 创建 `op-tui/src/ui/<name>.rs` — 游戏专用渲染
3. 修改 `op-tui/src/ui/mod.rs` — 在 Screen 分发中添加新游戏
4. 修改 `op-tui/src/app.rs` — GameState 中添加新游戏的状态解码和键盘处理

### TUI 规范（必须遵守）
- 所有屏幕底部必须有快捷键提示
- 所有屏幕必须支持 Esc/Q 返回上一屏
- F12 循环日志面板
- 使用相对座位计算模式（`my_idx` 为基准）

### 交付物
- TUI 渲染代码
- 编译通过

### ⏸️ 停止并等待开发者 review

---

## Phase 5 (可选): 集成接入

### 目标
将游戏注册到 op-host，包括 Bot 支持。

### 步骤
1. 在 `op-host/Cargo.toml` 添加游戏 crate 依赖
2. 在 `op-host/src/main.rs` 中：
   - 添加游戏实例化分支（根据 `config.app` 判断）
   - 实现 `<Name>BotProgram`（impl `UserProgram` trait）
   - 实现 `<Name>BotFactory`（impl `BotFactory` trait）
3. 可选：在 `games/<name>/src/bot.rs` 中实现更智能的 Bot 策略

### 交付物
- op-host 集成代码
- 编译通过

### ⏸️ 停止并等待开发者 review

---

## 附录 A: 框架技术参考

以下是实现时需要遵循的框架 API 和约定。

### A.1 Game Trait

```rust
pub trait Game: Send + Sync + 'static {
    // 必须实现
    fn meta(&self) -> GameMeta;
    fn handle_action(&mut self, ctx: &RoomContext, event: SequencedGameUpdate) -> GameUpdate;
    fn current_view(&self, ctx: &RoomContext) -> Option<GameUpdate>;

    // 可选（有默认实现）
    fn default_action(&self, player_id: &UserId) -> Option<Action> { None }
    fn apply_config(&mut self, config: TypedData) -> Result<(), String> { Ok(()) }
}
```

| 方法 | 职责 |
|------|------|
| `meta()` | 返回 `GameMeta { app: App { id, revision }, description }` |
| `handle_action()` | 处理所有游戏事件（GameStart/Action/TimerExpired/Interval），返回 GameUpdate |
| `current_view()` | 生成当前状态快照（不修改状态），用于重连。游戏未进行时返回 None |
| `default_action()` | 超时 fallback：返回当前玩家的默认动作。用于超时自动操作 |
| `apply_config()` | 运行时配置更新 |

### A.2 关键类型

#### 输入侧

| 类型 | 说明 |
|------|------|
| `SequencedGameUpdate { event: GameEvent, seq: u32 }` | 带序列号的事件包装 |
| `GameEvent` | 枚举：`Action(Action)`, `TimerExpired(TimeExpired)`, `Interval(Interval)`, `GameStart` |
| `Action { source: ActionSource, data: ActionData }` | 用户动作 |
| `ActionSource` | 枚举：`User(UserId)`, `System`。serde tag: `#[serde(tag = "type", content = "data")]` |
| `ActionData` | 枚举：`RoomAction(RoomActionData)`, `GameAction(GameActionData)`。serde: `#[serde(tag = "action_type", content = "data")]` |
| `GameActionData { message: TypedData, ref_version: u32 }` | 游戏动作载荷 + 乐观锁版本号 |

#### 输出侧

| 类型 | 说明 |
|------|------|
| `GameUpdate { views, snapshot, commands }` | handle_action 的返回值 |
| `views: HashMap<RoomView, GameViewUpdate>` | 各视角的视图更新 |
| `GameViewUpdate { events: Vec<ClientEvent>, new_view: GameState }` | 发给客户端的更新 |
| `GameState { version: u32, data: TypedData }` | 带版本号的序列化游戏状态 |
| `ClientEvent { seq: u32, message: TypedData }` | 客户端事件 |
| `GameCommand` | 枚举：`CreateTimer{id,duration}`, `CancelTimer{id,duration}`, `CreateInterval{id}`, `CancelInterval{id}`, `GameOver` |

#### 房间/视图

| 类型 | 说明 |
|------|------|
| `RoomContext { pub room: Room }` | 游戏运行时上下文 |
| `RoomView` | 枚举：`Position(RoomPlayerPosition)`, `Neutral` |
| `RoomPlayerPosition(String)` | 座位标识，如 "0", "1", "2" |

### A.3 Cargo.toml 模板

```toml
[package]
name = "openplay-<name>"
version = "0.1.0"
edition = "2021"

[dependencies]
openplay-basic = { path = "../../models/basic" }
# openplay-poker = { path = "../../models/poker" }  # 如果是扑克类游戏
bytes = "1.0"
chrono = { version = "0.4", features = ["serde"] }
rand = "0.9"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.21.0", features = ["v4", "fast-rng"] }
tracing = "0.1"

[dev-dependencies]
tokio = { version = "1.28", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

Workspace 的 `Cargo.toml` 使用 `members = ["games/*", ...]` 通配符，新 crate 自动包含。

### A.4 常量约定

每个游戏 crate 必须定义：

```rust
pub const APP_ID: &str = "<name>";          // 小写，与 crate 名对应
pub const APP_REVISION: u32 = 1;

pub fn get_app() -> App {
    App { id: APP_ID.to_string(), revision: APP_REVISION }
}
```

### A.5 游戏状态 Struct 设计模式

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyGame {
    pub config: MyGameConfig,
    pub version: u32,                    // 乐观锁，每次成功状态变更 +1
    pub players: Vec<PlayerState>,
    pub stage: Stage,
    pub current_turn: usize,

    // 客户端倒计时用：序列化给客户端
    #[serde(default)]
    pub turn_deadline: Option<i64>,      // Unix 毫秒时间戳

    // 服务端运行时：不序列化到客户端
    #[serde(skip)]
    pub timer_id: Option<Id>,
}
```

关键模式：
- `version` 每次成功状态变更递增，用于乐观锁
- `turn_deadline` 序列化给客户端做倒计时显示
- `timer_id` 用 `#[serde(skip)]`，服务端运行时管理

### A.6 handle_action 实现模式

```rust
fn handle_action(&mut self, ctx: &RoomContext, update: SequencedGameUpdate) -> GameUpdate {
    // 1. GameStart 事件
    if let GameEvent::GameStart = update.event {
        self.start(ctx);     // 从 ctx.get_ordered_players() 获取玩家列表
        let cmds = self.start_turn_timer();
        return self.make_update(ctx, vec![], cmds);
    }

    // 2. 分发处理
    let (events, commands) = match update.event {
        GameEvent::Action(action) => self.handle_user_action(ctx, action),
        GameEvent::TimerExpired(timer) => self.handle_timer_expired(ctx, timer),
        _ => (vec![], vec![]),
    };

    // 3. 空结果 = 动作被拒绝，返回空 views（防止 bot 无限循环）
    if events.is_empty() && commands.is_empty() {
        return GameUpdate {
            views: HashMap::new(),
            snapshot: GameState { version: self.version, data: self.snapshot() },
            commands: vec![],
        };
    }

    self.make_update(ctx, events, commands)
}
```

handle_user_action 内部流程：
1. 验证 `ActionSource` 是 `User`
2. 验证 `ActionData` 是 `GameAction`
3. 乐观锁检查 `ref_version == self.version`
4. `serde_json::from_slice` 反序列化游戏特定 Action
5. 验证是当前回合玩家
6. 调用具体处理方法
7. 成功时 `self.version += 1`

### A.7 视图生成模式

```rust
fn make_update(&self, _ctx: &RoomContext, events: Vec<ClientEvent>, commands: Vec<GameCommand>) -> GameUpdate {
    let mut views = HashMap::new();

    // 每个玩家位置的遮蔽视图
    for (i, _) in self.players.iter().enumerate() {
        let pos = RoomPlayerPosition::from(i.to_string());
        views.insert(RoomView::Position(pos), GameViewUpdate {
            events: events.clone(),
            new_view: GameState { version: self.version, data: self.masked_snapshot(Some(i)) },
        });
    }

    // 观察者视图
    views.insert(RoomView::Neutral, GameViewUpdate {
        events: events.clone(),
        new_view: GameState { version: self.version, data: self.masked_snapshot(None) },
    });

    GameUpdate {
        views,
        snapshot: GameState { version: self.version, data: self.snapshot() },
        commands,
    }
}
```

遮蔽逻辑（masked_snapshot）模式：
1. `serde_json::to_value(&self)` 序列化为 JSON Value
2. 遍历修改 Value，清除对该视角不可见的字段
3. 封装为 TypedData 返回

### A.8 TypedData 封装约定

```rust
fn snapshot(&self) -> TypedData {
    let json = serde_json::to_vec(self).unwrap();
    TypedData {
        r#type: DataType {
            app: get_app(),
            r#type: "<name>_state".to_string(),   // 如 "holdem_state"
        },
        codec: "json".to_string(),
        data: Data(Bytes::from(json)),
    }
}
```

动作也用 TypedData 封装：
```rust
TypedData {
    r#type: DataType { app: get_app(), r#type: "action".to_string() },
    codec: "json".to_string(),
    data: Data(Bytes::from(serde_json::to_vec(&action).unwrap())),
}
```

### A.9 序列化约定

| 类型 | JSON 格式 | 说明 |
|------|-----------|------|
| `UserId(Bytes)` | UTF-8 字符串（如 `"alice"`） | 回退 base64 |
| `Data(Bytes)` | base64 字符串 | 仅 HTTP/SSE 传输；内存传递时 `.0` 直接是原始 bytes |
| `TypedData` | `{ "type": {...}, "codec": "json", "data": "<base64>" }` | 消息信封 |
| `ActionData` | `#[serde(tag = "action_type", content = "data")]` | |
| `ActionSource` | `#[serde(tag = "type", content = "data")]` | |

客户端解码流程：
```
收到 GameViewUpdate
→ new_view.data 是 TypedData
→ TypedData.data.0 是游戏状态的 JSON bytes
→ serde_json::from_slice::<MyGame>() 反序列化
```

### A.10 Timer 管理

```rust
fn start_turn_timer(&mut self) -> Vec<GameCommand> {
    let mut commands = Vec::new();

    // 取消已有 timer
    if let Some(timer_id) = self.timer_id.take() {
        commands.push(GameCommand::CancelTimer { id: timer_id, duration: Duration::ZERO });
    }

    // 创建新 timer
    let new_id = Id::from(uuid::Uuid::new_v4().to_string());
    self.timer_id = Some(new_id.clone());
    commands.push(GameCommand::CreateTimer {
        id: new_id,
        duration: Duration::from_secs(self.config.turn_timeout_seconds),
    });

    // 设置客户端倒计时
    self.turn_deadline = Some(chrono::Utc::now().timestamp_millis() + timeout_ms);

    commands
}
```

Timer 到期处理时必须验证 timer_id 匹配，避免处理已取消的 timer。
游戏结束时必须取消 timer 并发送 `GameCommand::GameOver`。

### A.11 Bot 接口

Bot 通过 `UserProgram` trait 工作（在 `user-agents/programmed/` 中定义）：

```rust
pub trait UserProgram: Send + Sync + 'static {
    fn decide(&self, update: &GameViewUpdate) -> Option<TypedData>;
}
```

- Bot 只接收 `GameViewUpdate`（遮蔽视图，与人类玩家完全一样）
- Bot 返回 `Option<TypedData>`（游戏 action 的 TypedData 封装）
- Bot 有版本去重机制，相同 version 不重复处理

在 op-host 中需实现：
- `<Name>BotProgram`：impl `UserProgram`，解码游戏状态 → 调用决策逻辑 → 编码返回
- `<Name>BotFactory`：impl `BotFactory`，创建 Bot User（`is_bot: true`）和 `DynUserAgent`

---

## 附录 B: 项目必须遵守的规则

- **不使用 `async_trait` 宏** — 使用原生 Rust 模式
- **i18n 用 Fluent** — `fluent` 0.17，`FluentBundle::new_concurrent()`
- **游戏启动只能由 owner 发 `RoomManage::StartGame`** — 不会因全部 ready 自动开始
- **加入房间需显式 `Join` action**
- **RoomContext 含 Room 数据** — 游戏通过 `ctx.get_ordered_players()` 获取按座位排序的玩家列表
- **`RoomPlayerState` 字段名 `id_ready`** — 这是已有代码的 typo（应为 `is_ready`），不要改它
- **Bot 只处理 `Update::GameView`** — Bot 不能发 RoomAction。Bot 插入时自动 ready

---

## 附录 C: 集成点 Checklist

新游戏完整接入需修改的位置：

| 位置 | 修改内容 | Phase |
|------|----------|-------|
| `games/<name>/` | 新 crate（Game trait 实现） | Phase 2 |
| `games/<name>/INSTRUCTION.md` | 游戏规格说明书 | Phase 1 |
| `op-host/Cargo.toml` | 添加游戏 crate 依赖 | Phase 5 |
| `op-host/src/main.rs` | 游戏实例化 + BotProgram + BotFactory | Phase 5 |
| `op-tui/Cargo.toml` | 添加游戏 crate 依赖 | Phase 4 |
| `op-tui/src/ui/<name>.rs` | 游戏 UI 渲染 | Phase 4 |
| `op-tui/src/ui/mod.rs` | Screen 分发添加新游戏 | Phase 4 |
| `op-tui/src/app.rs` | GameState 解码 + 键盘处理 | Phase 4 |

**不需要修改** 的文件：
- `host/src/service.rs` — 使用 `DynGame = Box<dyn Game>` 类型擦除，完全游戏无关
- `models/basic/` — 通用框架层
- `Cargo.toml`（workspace 根） — `games/*` 通配符自动包含新 crate

---

## 附录 D: 关键文件速查

| 文件 | 用途 |
|------|------|
| `models/basic/src/game.rs` | Game trait, GameEvent, GameUpdate, GameCommand 等 |
| `models/basic/src/room.rs` | Room, RoomContext, RoomView, RoomPlayerPosition |
| `models/basic/src/message.rs` | TypedData, App, DataType |
| `models/basic/src/data.rs` | Data 序列化（base64） |
| `models/basic/src/user.rs` | User, UserId, Action, ActionData, ActionSource |
| `models/basic/src/user/game_action.rs` | GameActionData |
| `games/doudizhu/src/lib.rs` | 参考实现：DouDizhuGame（833 行） |
| `host/src/service.rs` | RoomService（游戏无关的通用服务层） |
| `user-agents/programmed/src/lib.rs` | UserProgram trait（Bot 接口） |
| `op-host/src/main.rs` | 游戏注册、BotProgram/BotFactory 实现 |
| `op-tui/src/ui/doudizhu.rs` | 参考实现：TUI 游戏渲染 |
| `op-tui/src/app.rs` | TUI 状态管理、GameState、键盘处理 |
