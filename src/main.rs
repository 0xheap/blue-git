mod agent;
mod config;
mod providers;
mod tools;

use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;
use rustyline::DefaultEditor;

use crate::agent::Agent;
use crate::config::{AppConfig, AppState, ProviderKind};

fn load_skill_prompt() -> Option<String> {
    std::fs::read_to_string("src/skills/SKILL.md").ok()
}

fn prompt(state: &AppState, cfg: &AppConfig) -> String {
    format!(
        "[{}/{}] {} > ",
        state.active_provider.as_str().cyan(),
        state.current_model(cfg).yellow(),
        state.repo_dir.display().to_string().green()
    )
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cfg = AppConfig::from_env()?;
    let mut state = AppState {
        active_provider: cfg.default_provider,
        model_overrides: Default::default(),
        repo_dir: std::env::current_dir()?,
    };
    let mut agent = Agent::new(load_skill_prompt());
    let client = reqwest::Client::new();
    let mut rl = DefaultEditor::new()?;

    println!("bluegit ready. /exit to quit.");

    loop {
        let p = prompt(&state, &cfg);
        let line = match rl.readline(&p) {
            Ok(s) => s.trim().to_string(),
            Err(_) => break,
        };
        if line.is_empty() {
            continue;
        }
        let _ = rl.add_history_entry(line.as_str());

        if line.starts_with('/') {
            if !handle_slash(&line, &mut state, &mut agent, &cfg)? {
                break;
            }
            continue;
        }

        match agent.run_turn(line, &state, &cfg, &client).await {
            Ok(out) => println!("{}", out),
            Err(e) => eprintln!("agent error: {}", e),
        }
    }

    Ok(())
}

fn handle_slash(
    cmd: &str,
    state: &mut AppState,
    agent: &mut Agent,
    cfg: &AppConfig,
) -> Result<bool> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    match parts.first().copied().unwrap_or_default() {
        "/exit" => Ok(false),
        "/auth" => {
            let provider = parts.get(1).copied().unwrap_or_default();
            if let Some(p) = ProviderKind::parse(provider) {
                if cfg.providers.contains_key(&p) {
                    state.active_provider = p;
                    println!("provider => {}", p.as_str());
                } else {
                    println!(
                        "provider `{}` is not configured (missing API key in .env)",
                        p.as_str()
                    );
                }
            } else {
                println!("usage: /auth openrouter|inception|bigmodel|zhipu|mistral");
            }
            Ok(true)
        }
        "/model" => {
            let model = parts.get(1).copied().unwrap_or_default();
            if model.is_empty() {
                println!("usage: /model <name>");
            } else {
                state
                    .model_overrides
                    .insert(state.active_provider, model.to_string());
                println!("model => {}", model);
            }
            Ok(true)
        }
        "/cd" => {
            let path = parts.get(1).copied().unwrap_or_default();
            if path.is_empty() {
                println!("usage: /cd <path>");
            } else {
                let next = if path.starts_with('/') {
                    PathBuf::from(path)
                } else {
                    state.repo_dir.join(path)
                };
                if next.is_dir() {
                    state.repo_dir = next.canonicalize()?;
                    println!("repo => {}", state.repo_dir.display());
                } else {
                    println!("not a directory: {}", next.display());
                }
            }
            Ok(true)
        }
        "/clear" => {
            agent.clear();
            println!("conversation cleared");
            Ok(true)
        }
        "/history" => {
            for msg in agent.history_view(10) {
                println!("{}", msg);
            }
            Ok(true)
        }
        "/tools" => {
            for tool in tools::definitions() {
                if let Some(name) = tool
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                {
                    println!("{}", name);
                }
            }
            Ok(true)
        }
        _ => {
            println!("unknown command");
            Ok(true)
        }
    }
}
