use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderKind {
    OpenRouter,
    Inception,
    /// 智谱 AI Open Platform (BigModel), OpenAI-compatible chat completions.
    BigModel,
}

impl ProviderKind {
    pub fn parse(input: &str) -> Option<Self> {
        match input.trim().to_lowercase().as_str() {
            "openrouter" => Some(Self::OpenRouter),
            "inception" => Some(Self::Inception),
            "bigmodel" | "zhipu" => Some(Self::BigModel),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OpenRouter => "openrouter",
            Self::Inception => "inception",
            Self::BigModel => "bigmodel",
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
        let mut providers = HashMap::new();

        if let Ok(k) = env::var("OPENROUTER_API_KEY") {
            if !k.trim().is_empty() {
                providers.insert(
                    ProviderKind::OpenRouter,
                    ProviderConfig {
                        api_key: k,
                        base_url: "https://openrouter.ai/api/v1/chat/completions".to_string(),
                        default_model: "anthropic/claude-sonnet-4-5".to_string(),
                    },
                );
            }
        }

        if let Ok(k) = env::var("INCEPTION_API_KEY") {
            if !k.trim().is_empty() {
                providers.insert(
                    ProviderKind::Inception,
                    ProviderConfig {
                        api_key: k,
                        base_url: "https://api.inceptionlabs.ai/v1/chat/completions".to_string(),
                        default_model: "mercury-coder-small".to_string(),
                    },
                );
            }
        }

        let bigmodel_key = env::var("BIGMODEL_API_KEY")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| env::var("ZHIPU_API_KEY").ok().filter(|s| !s.trim().is_empty()));

        if let Some(k) = bigmodel_key {
            let base_url = env::var("BIGMODEL_CHAT_COMPLETIONS_URL").unwrap_or_else(|_| {
                let coding = env::var("BIGMODEL_USE_CODING_ENDPOINT")
                    .ok()
                    .map(|v| {
                        matches!(
                            v.trim().to_ascii_lowercase().as_str(),
                            "1" | "true" | "yes"
                        )
                    })
                    .unwrap_or(false);
                if coding {
                    "https://open.bigmodel.cn/api/coding/paas/v4/chat/completions".to_string()
                } else {
                    "https://open.bigmodel.cn/api/paas/v4/chat/completions".to_string()
                }
            });

            providers.insert(
                ProviderKind::BigModel,
                ProviderConfig {
                    api_key: k,
                    base_url,
                    default_model: "glm-5.1".to_string(),
                },
            );
        }

        if providers.is_empty() {
            return Err(anyhow!(
                "No API keys configured. Set at least one of: OPENROUTER_API_KEY, INCEPTION_API_KEY, BIGMODEL_API_KEY (or ZHIPU_API_KEY)."
            ));
        }

        let requested = env::var("DEFAULT_PROVIDER")
            .ok()
            .and_then(|v| ProviderKind::parse(&v));

        let default_provider = match requested {
            Some(p) if providers.contains_key(&p) => p,
            Some(p) => {
                return Err(anyhow!(
                    "DEFAULT_PROVIDER {:?} has no API key or unknown name. Configured providers: {}",
                    p,
                    list_provider_names(&providers)
                ));
            }
            None => pick_default_provider(&providers),
        };

        Ok(Self {
            providers,
            default_provider,
        })
    }
}

fn list_provider_names(providers: &HashMap<ProviderKind, ProviderConfig>) -> String {
    let mut names: Vec<&'static str> = providers.keys().map(|k| k.as_str()).collect();
    names.sort();
    names.join(", ")
}

fn pick_default_provider(providers: &HashMap<ProviderKind, ProviderConfig>) -> ProviderKind {
    for candidate in [
        ProviderKind::OpenRouter,
        ProviderKind::Inception,
        ProviderKind::BigModel,
    ] {
        if providers.contains_key(&candidate) {
            return candidate;
        }
    }
    unreachable!("providers map was validated non-empty")
}
