use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{AppConfig, ProviderKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: ChatMessage,
}

pub async fn chat_completion(
    client: &Client,
    cfg: &AppConfig,
    provider: ProviderKind,
    model: &str,
    messages: &[ChatMessage],
    tools: &[Value],
) -> Result<ChatMessage> {
    let p = cfg
        .providers
        .get(&provider)
        .ok_or_else(|| anyhow!("Provider config missing"))?;

    // Omit `tool_choice`: OpenAI defaults to "auto" when tools are present, and some
    // OpenRouter routes error if `tool_choice` is set but no provider supports it.
    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "tools": tools,
    });

    let mut req = client
        .post(&p.base_url)
        .bearer_auth(&p.api_key)
        .header("content-type", "application/json");

    if provider == ProviderKind::OpenRouter {
        req = req.header("HTTP-Referer", "https://github.com/blue-git-agent");
    }

    let resp = req
        .json(&body)
        .send()
        .await
        .context("Failed to send chat completion request")?;
    let status = resp.status();
    let text = resp.text().await.context("Failed reading provider body")?;

    if !status.is_success() {
        let mut err = format!("Provider HTTP {}: {}", status, text);
        if provider == ProviderKind::OpenRouter
            && (text.contains("tool_choice") || text.contains("No endpoints found"))
        {
            err.push_str(
                "\n\nOpenRouter: this model has no endpoint that supports **tool/function calling**, which bluegit requires. \
Your API key is fine — pick a model that lists `tools` on OpenRouter, e.g. browse: \
https://openrouter.ai/models?supported_parameters=tools\n\
Example models: `anthropic/claude-sonnet-4.5`, `openai/gpt-4o`. Many `:free` models do not support tools.",
            );
        }
        return Err(anyhow!("{}", err));
    }

    let parsed: ChatCompletionResponse =
        serde_json::from_str(&text).context("Failed to parse provider response JSON")?;
    let message = parsed
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Provider returned no choices"))?
        .message;

    Ok(message)
}
