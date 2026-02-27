use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(name = "op-tui", about = "OpenPlay TUI client")]
pub struct CliArgs {
    /// Path to config file
    #[arg(long)]
    pub config: Option<String>,

    /// Server URL (e.g. http://localhost:3000)
    #[arg(long)]
    pub server_url: Option<String>,

    /// User ID for authentication
    #[arg(long)]
    pub user_id: Option<String>,

    /// Locale (e.g. "en", "zh-CN")
    #[arg(long)]
    pub locale: Option<String>,

    /// Room API path (default: /room/ua)
    #[arg(long)]
    pub room_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    /// Server base URL
    #[serde(default = "default_server_url")]
    pub server_url: String,

    /// User ID (if not set, will prompt in lobby)
    #[serde(default)]
    pub user_id: Option<String>,

    /// Locale preference
    #[serde(default)]
    pub locale: Option<String>,

    /// Room API path
    #[serde(default = "default_room_path")]
    pub room_path: String,
}

fn default_server_url() -> String {
    "http://localhost:3000".to_string()
}

fn default_room_path() -> String {
    "/room/ua".to_string()
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            server_url: default_server_url(),
            user_id: None,
            locale: None,
            room_path: default_room_path(),
        }
    }
}

impl TuiConfig {
    /// Load config with layered priority: CLI > env > file > defaults
    pub fn load(args: &CliArgs) -> anyhow::Result<Self> {
        let mut builder = config::Config::builder();

        // Layer 1: defaults
        builder = builder
            .set_default("server_url", default_server_url())?
            .set_default("room_path", default_room_path())?;

        // Layer 2: config file (if specified)
        if let Some(ref path) = args.config {
            builder = builder.add_source(config::File::with_name(path).required(true));
        }

        // Layer 3: environment variables (OP_TUI_ prefix)
        builder = builder.add_source(
            config::Environment::with_prefix("OP_TUI")
                .separator("_")
                .try_parsing(true),
        );

        // Layer 4: CLI overrides
        if let Some(ref url) = args.server_url {
            builder = builder.set_override("server_url", url.clone())?;
        }
        if let Some(ref uid) = args.user_id {
            builder = builder.set_override("user_id", uid.clone())?;
        }
        if let Some(ref locale) = args.locale {
            builder = builder.set_override("locale", locale.clone())?;
        }
        if let Some(ref path) = args.room_path {
            builder = builder.set_override("room_path", path.clone())?;
        }

        let cfg = builder.build()?;
        let tui_config: TuiConfig = cfg.try_deserialize()?;
        Ok(tui_config)
    }
}
