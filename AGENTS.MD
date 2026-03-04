# LLM's Language usage

Answer user's question in what language they ask with.

# RUST 风格

尽量使用2024版本的rust风格，尽量避免魔法值，对于返回Error的类型，在注释的`#ERROR`章节说明错误可能产生的原因。

## 组织项目结构
不要使用`mod.rs`，而是使用`xxx.rs`，并建立`xxx`文件夹，这样的项目更好组织，也是最新的rust推荐方式

## axum http handler 集成规范：基于 Trait 的状态注入 (State Injection)

为了确保 user-agents 能够在不同的 Axum 应用中复用并保持高度的可配置性，我们遵循以下集成规范。

### 核心理念

我们使用 `axum::extract::State` 提取器，配合 Rust 的 Trait 系统，实现**依赖注入**。

组件库（如 `openplay-ua-http`）不直接依赖具体的数据结构（如 `AppState`），而是定义一个 **Trait**，声明它需要的能力（Capabilities）。

集成方（应用层）只需在其全局状态结构体上实现这个 Trait，即可无缝集成组件。

### 规范细节

#### 1. 组件方 (Component Provider)

组件库必须暴露以下内容：

1.  **配置结构体 (`Config`)**:
    - 纯数据结构 (POJO)。
    - 包含运行时参数（超时、容量、缓冲区大小等）。
    - 必须实现 `Default` 和 `Clone`。

    ```rust
    #[derive(Debug, Clone)]
    pub struct AgentConfig {
        pub broadcast_capacity: usize,
        pub keep_alive_interval: Duration,
    }
    ```

2.  **状态接口 (`State Trait`)**:
    - 定义获取依赖的方法。
    - 必须继承 `Send + Sync + 'static`。

    ```rust
    pub trait AgentState: Send + Sync + 'static {
        /// 获取组件配置
        fn config(&self) -> &AgentConfig;
        /// 获取组件所需的特定资源（如注册表）
        fn registry(&self) -> &Registry;
    }
    ```

3.  **泛型处理器 (Generic Handlers)**:
    - Handler 函数必须使用泛型 `S`。
    - `S` 必须约束为实现了 `AgentState`。

    ```rust
    pub async fn handler<S>(
        State(state): State<S>,
    ) -> impl IntoResponse
    where
        S: AgentState,
    {
        let config = state.config();
        // ...
    }
    ```

#### 2. 集成方 (Application Consumer)

应用层需要做三件事：

1.  **定义全局状态**:

    ```rust
    #[derive(Clone)]
    struct AppState {
        db: DatabaseConnection,
        agent_registry: Registry, // 组件依赖
        agent_config: AgentConfig, // 组件配置
    }
    ```

2.  **实现 Trait**:

    ```rust
    impl AgentState for AppState {
        fn config(&self) -> &AgentConfig {
            &self.agent_config
        }
        fn registry(&self) -> &Registry {
            &self.agent_registry
        }
    }
    ```

3.  **挂载路由**:

    ```rust
    let state = AppState { ... };
    let app = Router::new()
        .route("/events", get(openplay_ua_http::sse_handler))
        .with_state(state);
    ```

### 优势

- **解耦**: 组件不关心 `AppState` 中还有什么（数据库、缓存等）。
- **可测试性**: 单元测试时可以轻松 mock 一个实现了 Trait 的简单状态，无需构建复杂的应用上下文。
- **灵活性**: 配置可以在运行时加载（例如从环境变量或配置中心），而不是硬编码在组件内部。

## 文档索引

| 文件 | 说明 |
|---|---|
| `design.md` | 整体架构设计：用户身份模型、Room 机制、Action/Update 流程 |
| `client/USER_IDENTITY.md` | 用户身份存储格式、默认路径、API 用法、认证流程及 op-tui 集成说明 |
| `games/doudizhu/INSTRUCTION.md` | 斗地主游戏规格：状态机、出牌规则、计时器、视图掩码、客户端快捷键 |

# MCP
You are able to use the Svelte MCP server, where you have access to comprehensive Svelte 5 and SvelteKit documentation. Here's how to use the available tools effectively:

## Available MCP Tools:

### 1. list-sections

Use this FIRST to discover all available documentation sections. Returns a structured list with titles, use_cases, and paths.
When asked about Svelte or SvelteKit topics, ALWAYS use this tool at the start of the chat to find relevant sections.

### 2. get-documentation

Retrieves full documentation content for specific sections. Accepts single or multiple sections.
After calling the list-sections tool, you MUST analyze the returned documentation sections (especially the use_cases field) and then use the get-documentation tool to fetch ALL documentation sections that are relevant for the user's task.

### 3. svelte-autofixer

Analyzes Svelte code and returns issues and suggestions.
You MUST use this tool whenever writing Svelte code before sending it to the user. Keep calling it until no issues or suggestions are returned.

### 4. playground-link

Generates a Svelte Playground link with the provided code.
After completing the code, ask the user if they want a playground link. Only call this tool after user confirmation and NEVER if code was written to files in their project.