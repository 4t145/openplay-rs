# Godot 4 客户端施工文档

本文件描述 Godot 4 客户端的施工路线、接口约定与里程碑。

## 目标
- Godot 4 作为 UI 与交互层
- Rust 作为 GDExtension 扩展层，负责网络与协议
- 支持两种连接模式：direct（HTTP + SSE）与 steam（Lobby ID）
- 认证基于 user_id，steam 仅作为传输通道

## 范围
- 客户端 UI 流程
- Godot <-> Rust 扩展接口约定
- 连接与认证流程

非目标：服务端实现细节、Steam SDK 具体绑定代码。

## 总体架构
- Godot 4：UI、输入、状态展示、动画、音效
- Rust GDExtension：传输层与协议封装

```
Godot UI
  -> Rust Extension (Network Bridge)
     -> direct: HTTP + SSE
     -> steam: Steam Networking (Lobby ID)
```

## 连接模式
### direct
- 输入 server_url
- 建立 SSE 连接
- 连接成功后立即进入挑战认证

### steam
- 输入 Lobby ID
- 通过 Steam Networking 连接房主
- 连接成功后立即进入挑战认证

## 认证流程（两种模式一致）
1) 连接建立
2) 发起挑战请求
3) 使用本地 KeyPair 签名
4) 服务端返回 token
5) 后续请求携带 token

## Godot UI 流程
1) 启动页
2) 选择连接方式（direct / steam）
3) direct: 输入 server_url
4) steam: 输入 Lobby ID
5) 显示连接中 / 认证中状态
6) 进入房间 / 失败返回

## Rust Extension 对外接口（建议）
Godot 侧通过一个统一对象调用网络接口。

```
set_mode(mode)                 # "direct" | "steam"
connect_direct(server_url)
connect_steam(lobby_id)
send_action(bytes)
poll_events() -> [Event]
disconnect()
```

Event 约定：
- Connected
- Disconnected(reason)
- Message(bytes)
- Authenticated
- AuthFailed(error)

## 客户端本地身份
- 使用 openplay-client 的默认身份目录
- 不自定义存储格式
- 选择身份后用于认证与昵称

## UI 要点
- 连接方式显式选择，不做自动 fallback
- 显示当前连接方式与认证进度
- 连接失败与认证失败要区分

## 里程碑
1) Godot 4 与 Rust GDExtension 最小调用跑通
2) direct 模式连通 + 认证
3) steam 模式连通 + 认证
4) UI 流程完善 + 错误提示

## 建议目录结构
```
ui/godot/
  CONSTRUCTION.md
  godot_project/        # Godot 工程
  rust_ext/             # GDExtension 工程
```

## 验收清单
- direct 模式能连接 + 认证成功
- steam 模式能连接 + 认证成功
- 断线后能返回选择界面
- UI 明确显示连接方式与错误原因
