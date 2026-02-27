# LLM's Language usage

Answer user's question in what language they ask with.

# RUST 风格

尽量使用2024版本的rust风格，尽量避免魔法值，对于返回Error的类型，在注释的`#ERROR`章节说明错误可能产生的原因。

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
