use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderKind {
    OpenRouter,
    Inception,
}

impl ProviderKind {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim().to_lowercase().as_str() {
            "openrouter" => Some(Self::OpenRouter),
            "inception" => Some(Self::Inception),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OpenRouter => "openrouter",
            Self::Inception => "inception",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub api_key: String,
    pub base_url: String,
    pub default_model: String,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub providers: HashMap<ProviderKind, ProviderConfig>,
    pub default_provider: ProviderKind,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub active_provider: ProviderKind,
    pub model_overrides: HashMap<ProviderKind, String>,
    pub repo_dir: PathBuf,
}

impl AppState {
    pub fn current_model<'a>(&'a self, cfg: &'a AppConfig) -> &'a str {
        self.model_overrides
            .get(&self.active_provider)
            .map(String::as_str)
            .unwrap_or_else(|| cfg.providers[&self.active_provider].default_model.as_str())
    }
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let openrouter_key = env::var("OPENROUTER_API_KEY")
            .map_err(|_| anyhow!("Missing OPENROUTER_API_KEY in environment"))?;
        let inception_key =
            env::var("INCEPTION_API_KEY").map_err(|_| anyhow!("Missing INCEPTION_API_KEY"))?;

        let default_provider = env::var("DEFAULT_PROVIDER")
            .ok()
            .and_then(|v| ProviderKind::parse(&v))
            .unwrap_or(ProviderKind::OpenRouter);

        let mut providers = HashMap::new();
        providers.insert(
            ProviderKind::OpenRouter,
            ProviderConfig {
                api_key: openrouter_key,
                base_url: "https://openrouter.ai/api/v1/chat/completions".to_string(),
                default_model: "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free".to_string(),
            },
        );
        providers.insert(
            ProviderKind::Inception,
            ProviderConfig {
                api_key: inception_key,
                base_url: "https://api.inceptionlabs.ai/v1/chat/completions".to_string(),
                default_model: "mercury-2".to_string(),
            },
        );

        Ok(Self {
            providers,
            default_provider,
        })
    }
}
