use std::path::Path;

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

pub mod git;
pub mod shell;

pub fn definitions() -> Vec<Value> {
    vec![
        tool("git_status", json!({})),
        tool("git_diff", json!({"type":"object","properties":{"staged":{"type":"boolean"}}})),
        tool("git_log", json!({"type":"object","properties":{"n":{"type":"integer"}}})),
        tool("git_add", json!({"type":"object","properties":{"files":{"type":"array","items":{"type":"string"}}},"required":["files"]})),
        tool("git_commit", json!({"type":"object","properties":{"message":{"type":"string"}},"required":["message"]})),
        tool("git_push", json!({"type":"object","properties":{"branch":{"type":"string"},"force":{"type":"boolean"},"confirmed":{"type":"boolean"}},"required":["branch"]})),
        tool("git_pull", json!({"type":"object","properties":{"rebase":{"type":"boolean"}}})),
        tool("git_branch", json!({"type":"object","properties":{"action":{"type":"string"},"name":{"type":"string"},"old":{"type":"string"},"new":{"type":"string"}}})),
        tool("git_checkout", json!({"type":"object","properties":{"target":{"type":"string"},"create":{"type":"boolean"}},"required":["target"]})),
        tool("git_stash", json!({"type":"object","properties":{"action":{"type":"string"},"message":{"type":"string"}}})),
        tool("git_reset", json!({"type":"object","properties":{"mode":{"type":"string"},"ref":{"type":"string"}}})),
        tool("git_revert", json!({"type":"object","properties":{"hash":{"type":"string"}},"required":["hash"]})),
        tool("git_cherry_pick", json!({"type":"object","properties":{"hash":{"type":"string"}},"required":["hash"]})),
        tool("git_show", json!({"type":"object","properties":{"ref":{"type":"string"}}})),
        tool("git_blame", json!({"type":"object","properties":{"file":{"type":"string"},"lines":{"type":"string"}},"required":["file"]})),
        tool("git_remote", json!({})),
        tool("git_fetch", json!({"type":"object","properties":{"prune":{"type":"boolean"}}})),
        tool("git_merge", json!({"type":"object","properties":{"branch":{"type":"string"},"no_ff":{"type":"boolean"}},"required":["branch"]})),
        tool("git_rebase", json!({"type":"object","properties":{"base":{"type":"string"},"interactive":{"type":"boolean"}},"required":["base"]})),
        tool("git_clean", json!({"type":"object","properties":{"dry_run":{"type":"boolean"}}})),
        tool("git_bisect", json!({"type":"object","properties":{"action":{"type":"string"},"hash":{"type":"string"}},"required":["action"]})),
        tool("read_file", json!({"type":"object","properties":{"path":{"type":"string"},"maxLines":{"type":"integer"}},"required":["path"]})),
        tool("cmd_execute", json!({"type":"object","properties":{"command":{"type":"string"},"cwd":{"type":"string"},"confirmed":{"type":"boolean"}},"required":["command"]})),
    ]
}

fn tool(name: &str, params: Value) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": name,
            "description": format!("Tool: {}", name),
            "parameters": params
        }
    })
}

pub async fn dispatch(name: &str, args: &Value, repo_dir: &Path) -> Result<String> {
    match name {
        "git_status" => git::git_status(repo_dir),
        "git_diff" => git::git_diff(repo_dir, args),
        "git_log" => git::git_log(repo_dir, args),
        "git_add" => git::git_add(repo_dir, args),
        "git_commit" => git::git_commit(repo_dir, args),
        "git_push" => git::git_push(repo_dir, args),
        "git_pull" => git::git_pull(repo_dir, args),
        "git_branch" => git::git_branch(repo_dir, args),
        "git_checkout" => git::git_checkout(repo_dir, args),
        "git_stash" => git::git_stash(repo_dir, args),
        "git_reset" => git::git_reset(repo_dir, args),
        "git_revert" => git::git_revert(repo_dir, args),
        "git_cherry_pick" => git::git_cherry_pick(repo_dir, args),
        "git_show" => git::git_show(repo_dir, args),
        "git_blame" => git::git_blame(repo_dir, args),
        "git_remote" => git::git_remote(repo_dir),
        "git_fetch" => git::git_fetch(repo_dir, args),
        "git_merge" => git::git_merge(repo_dir, args),
        "git_rebase" => git::git_rebase(repo_dir, args),
        "git_clean" => git::git_clean(repo_dir, args),
        "git_bisect" => git::git_bisect(repo_dir, args),
        "read_file" => shell::read_file(repo_dir, args),
        "cmd_execute" => shell::cmd_execute(repo_dir, args),
        _ => Err(anyhow!("Unknown tool {}", name)),
    }
}
