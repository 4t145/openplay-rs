# 用户身份存储说明

`openplay-client` 使用 **ed25519 密钥对**作为用户的持久身份。公钥即 `UserId`，私钥用于向服务端进行挑战-响应认证。

## 文件格式

每个身份以一个 JSON 文件存储：

```json
{
  "signing_key": "<base64-standard-encoded 32-byte ed25519 seed>",
  "user": {
    "nickname": "player",
    "avatar_url": null,
    "is_bot": false
  }
}
```

字段说明：

| 字段 | 类型 | 说明 |
|---|---|---|
| `signing_key` | string | ed25519 私钥 seed，标准 Base64 编码（32 字节）|
| `user.nickname` | string | 显示昵称 |
| `user.avatar_url` | string \| null | 头像 URL，可省略 |
| `user.is_bot` | bool | 是否为机器人账号，默认 `false` |

## 文件命名

文件名由公钥派生，格式为：

```
<base64url-no-pad(public_key_bytes)>.json
```

使用 URL-safe Base64、无 padding，避免文件系统中 `+`、`/`、`=` 字符的问题。同一目录下可共存多个身份文件。

## 默认存储路径

由 `default_user_dir()` 返回，各平台规则如下：

| 平台 | 路径 |
|---|---|
| Linux | `$XDG_DATA_HOME/openplay/user/`（通常 `~/.local/share/openplay/user/`）|
| macOS | `~/Library/Application Support/openplay/user/` |
| Windows | `%APPDATA%\openplay\user\` |

目录不存在时会自动递归创建（`load_or_create` 内部调用 `ensure_user_dir`）。

## API 概览

```rust
use openplay_client::{KeyPair, default_user_dir, load_or_create};

// 生成新身份（不保存）
let kp = KeyPair::generate("alice");

// 保存到指定路径
kp.save(Path::new("/path/to/identity.json"))?;

// 从指定路径加载
let kp = KeyPair::load(Path::new("/path/to/identity.json"))?;

// 获取 UserId（公钥字符串表示）
let user_id: UserId = kp.user_id();

// 获取默认存储目录
let dir: PathBuf = default_user_dir()?;

// 从目录加载第一个身份；目录为空则自动生成并保存
let kp = load_or_create(&dir, "player")?;
```

## 认证流程

身份文件仅在本地存储私钥，服务端只保存公钥（`UserId`）。每次连接时通过挑战-响应证明私钥持有权：

```
客户端                                    服务端
  │                                         │
  │  POST /room/auth/challenge              │
  │  { "user_id": "<public_key_base64>" }  │
  │ ──────────────────────────────────────► │
  │                                         │  生成随机 32 字节 challenge
  │  { "challenge": "<base64>" }           │
  │ ◄────────────────────────────────────── │
  │                                         │
  │  用私钥签名 challenge                   │
  │                                         │
  │  POST /room/auth/verify                 │
  │  { "user_id": "...",                   │
  │    "signature": "<base64>" }           │
  │ ──────────────────────────────────────► │  验证签名
  │                                         │
  │  { "token": "<JWT>" }                  │
  │ ◄────────────────────────────────────── │
  │                                         │
  │  后续请求携带 Authorization: Bearer <JWT>│
```

具体实现见 `client/src/auth.rs` 中的 `authenticate()` 函数。

## op-tui 集成

`op-tui` 通过以下配置项控制身份行为：

| 配置项 | CLI 参数 | 说明 |
|---|---|---|
| `key_file` | `--key-file <path>` | 指定身份 JSON 文件路径；不存在时自动创建 |
| `nickname` | `--nickname <name>` | 生成新身份时使用的昵称，默认 `"player"` |
| `user_id` | `--user-id <id>` | 手动指定 user_id，跳过密钥认证（兼容旧模式）|

未指定 `key_file` 时，`op-tui` 会尝试从 `default_user_dir()` 自动加载身份；仍失败则回退到旧的 Bearer token 模式。

## 安全注意事项

- 身份文件包含 **明文私钥**，请妥善保管，不要提交到版本控制。
- 私钥丢失即等同于身份丢失，服务端无法恢复（无密码重置机制）。
- 不同房间服务器之间，同一个密钥对产生的 `UserId` 是相同的——这是设计意图，方便跨服务器统一身份。
