use anyhow::Result;
use reqwest::Client;
use serde_json::Value;

use crate::config::{AppConfig, AppState};
use crate::providers::{chat_completion, ChatMessage};
use crate::tools;

const MAX_ITERS: usize = 10;

const SYSTEM_PROMPT: &str = "You are a Git AI agent with real tools that run shell commands.
For commit messages: read the diff yourself, never ask the user to describe changes. Use conventional commit format: <type>(<scope>): <imperative summary under 72 chars>.
For push: always preview what will be pushed before pushing.
For troubleshooting: always read git_status and git_log first, give one-sentence diagnosis then fix.
Never run git_reset --hard without warning.
Stop immediately if a secret file appears in the diff.
Prefer rebase=true when pulling.
Keep responses short and terminal-friendly, no markdown headers.";

pub struct Agent {
    pub history: Vec<ChatMessage>,
    skill_prompt: Option<String>,
}

impl Agent {
    pub fn new(skill_prompt: Option<String>) -> Self {
        let mut history = vec![ChatMessage {
            role: "system".to_string(),
            content: Some(SYSTEM_PROMPT.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }];
        if let Some(skill) = skill_prompt.clone() {
            history.push(ChatMessage {
                role: "system".to_string(),
                content: Some(format!(
                    "Loaded project skill instructions. Follow them when relevant.\n\n{}",
                    skill
                )),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }
        Self {
            history,
            skill_prompt,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::new(self.skill_prompt.clone());
    }

    pub fn history_view(&self, n: usize) -> Vec<String> {
        self.history
            .iter()
            .rev()
            .filter(|m| m.role != "system")
            .take(n)
            .map(|m| {
                format!(
                    "{}: {}",
                    m.role,
                    m.content
                        .clone()
                        .unwrap_or_else(|| "<tool call message>".to_string())
                )
            })
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub async fn run_turn(
        &mut self,
        user_input: String,
        state: &AppState,
        cfg: &AppConfig,
        client: &Client,
    ) -> Result<String> {
        self.history.push(ChatMessage {
            role: "user".to_string(),
            content: Some(user_input),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });

        let tool_defs = tools::definitions();
        for _ in 0..MAX_ITERS {
            let model = state.current_model(cfg).to_string();
            let assistant = chat_completion(
                client,
                cfg,
                state.active_provider,
                &model,
                &self.history,
                &tool_defs,
            )
            .await?;

            let calls = assistant.tool_calls.clone().unwrap_or_default();
            self.history.push(assistant.clone());

            if calls.is_empty() {
                return Ok(assistant.content.unwrap_or_default());
            }

            for call in calls {
                let parsed: Value = serde_json::from_str(&call.function.arguments)
                    .unwrap_or_else(|_| serde_json::json!({}));
                let result = tools::dispatch(&call.function.name, &parsed, &state.repo_dir)
                    .await
                    .unwrap_or_else(|e| format!("tool error: {}", e));
                self.history.push(ChatMessage {
                    role: "tool".to_string(),
                    content: Some(result),
                    name: Some(call.function.name),
                    tool_calls: None,
                    tool_call_id: Some(call.id),
                });
            }
        }

        Ok("Stopped after 10 tool iterations to prevent infinite loop.".to_string())
    }
}
