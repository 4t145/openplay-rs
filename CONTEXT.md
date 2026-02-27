# OpenPlay Holdem - AI 开发上下文

> 本文件供后续 AI 会话参考，记录项目架构、设计决策和当前状态。

## 项目架构

### Workspace 结构
- `models/basic/` — 核心模型：User, UserId, Room, RoomContext, Data, TypedData, Game trait, Action 等
- `games/doudizhu/` — 斗地主游戏逻辑（DouDizhuGame implements Game trait）
- `host/` — 房间服务层（RoomService：管理房间、处理 action、广播更新）
- `user-agents/http/` — HTTP/SSE 用户代理（SSE 推送 + POST action）
- `user-agents/programmed/` — 编程式用户代理（Bot 用）
- `op-host/` — 服务端可执行文件（axum HTTP 服务器）
- `op-tui/` — TUI 客户端（Ratatui + SSE + 键盘交互）

### 通信流程
1. 客户端 GET `/room/ua` → SSE 连接，Bearer token = UserId
2. 连接后客户端发 POST `RoomActionData::Join` 加入房间
3. 服务端推送 `Update::Room(RoomUpdate)` 和 `Update::GameView(GameViewUpdate)`
4. 客户端 POST `/room/ua` 发送 Action（GameAction / RoomAction）

### 关键类型与序列化
- `UserId(Bytes)` — JSON 序列化为 UTF-8 字符串（回退 base64）。可做 HashMap key。
- `Data(Bytes)` — JSON 序列化为 base64 字符串（`is_human_readable()`），二进制格式为原始字节。
- `TypedData { type: DataType, codec: String, data: Data }` — 消息信封。data 是 base64 编码的内层 JSON。
- 客户端解码流程：parse outer JSON → 取 `data` 字段 → base64 decode → `serde_json::from_slice::<DouDizhuGame>()`
- `RoomContext { pub room: Room }` — 游戏运行时可访问房间信息。`get_ordered_players()` 按座位排序。
- `ActionData` serde: `#[serde(tag = "action_type", content = "data")]`
- `RoomActionData` serde: `#[serde(tag = "data_type", content = "data")]`
- `Update` serde: 默认 external tagging: `{"Room": {...}}` 或 `{"GameView": {...}}`

## 设计决策

### 必须遵守
- **No `async_trait` 宏** — 使用原生 Rust 模式（BoxFuture 可以）
- **i18n 用 Fluent** — `fluent` 0.17 meta-crate，`FluentBundle::new_concurrent()`，文件在 `locales/{en,zh-CN}/`
- **游戏启动只能由 owner 发 `RoomManage::StartGame`** — 不会因全部 ready 自动开始
- **Owner 是真实用户**（如 "alice"），不是 robot。`--owner_id` CLI 参数指定。
- **加入房间需显式 `Join` action** — TUI 在 SSE 连接后自动发送
- **RoomContext 含 Room 数据** — 游戏可通过 ctx 读取房间信息（谁在哪个座位等）
- **RoomPlayerState 字段名 `id_ready`** — 这是原代码的 typo（应为 `is_ready`），不要改它除非做全局重构
- **Bot 只能处理 `Update::GameView`** — 不能发 RoomAction。Bot 插入时自动 ready。

### op-tui 交互规范
- **所有屏幕底部必须有快捷键提示**
- **所有屏幕必须支持返回上一屏**（Esc/Q 返回，Ctrl+C 全局退出）
- **F12** 循环日志面板：Off→Panel→Fullscreen→Off（所有屏幕都可用）
- **Lobby**: 输入框 Tab 切换，Enter 连接
- **Waiting Room**: 1-3 选座，A+1-3 加 Bot，K+1-3 踢人，R ready，S 开始
- **Game**: 方向键选牌，Space 选中，Enter 出牌，P 过牌，B+0-3 叫分

## 当前状态（2026-02-27）

### 已完成
- 所有已知编译错误已修复，`cargo build --workspace` 和 `cargo test --workspace` 全部通过
- UserId 序列化（bytes→string）、Data 序列化（bytes→base64）
- RoomContext 从空 struct 改为含 `pub room: Room`
- 游戏 start() 从 RoomContext 读取玩家列表
- handle_timer_expired 正确传递 RoomContext
- TUI: Join 自动发送、F12 日志面板、UserId 字符串比较
- Service: Join handler、non-owner StartGame 警告

### 可能需要后续关注
- 大量 unused import warnings（不影响功能，可以清理）
- 端到端实际运行测试（启动 op-host + op-tui 联调）
- 斗地主 UI 的实际游戏操作（出牌、叫分）是否端到端工作
- Bot AI 决策逻辑（当前是随机/简单策略）
- 错误处理和断线重连的健壮性

## 关键文件速查

| 文件 | 用途 |
|------|------|
| `models/basic/src/data.rs` | Data 序列化（base64） |
| `models/basic/src/user.rs` | UserId 序列化（UTF-8 string） |
| `models/basic/src/room.rs` | Room, RoomContext 定义 |
| `models/basic/src/game.rs` | Game trait, GameEvent, GameUpdate |
| `models/basic/src/user/room_action.rs` | RoomActionData（Join, PositionChange, Ready 等） |
| `games/doudizhu/src/lib.rs` | DouDizhuGame 完整实现 |
| `host/src/service.rs` | RoomService（核心业务逻辑） |
| `host/src/connection.rs` | ConnectionController（连接管理） |
| `user-agents/http/src/lib.rs` | SSE handler + action handler |
| `op-host/src/main.rs` | 服务端入口（Bot 逻辑也在这） |
| `op-host/src/lib.rs` | run_server()、AppState |
| `op-tui/src/app.rs` | TUI 主逻辑（状态机、事件处理） |
| `op-tui/src/client.rs` | GameClient（SSE 连接、发送 action） |
| `op-tui/src/ui/doudizhu.rs` | 斗地主 UI 渲染 |
