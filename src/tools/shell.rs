use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Result};
use serde_json::Value;

const HARD_DENY: &[&str] = &[
    "rm -rf /",
    "rm -rf ~",
    "rm -rf /*",
    "mkfs",
    "dd if=",
    ":(){ :|:& };:",
    "shutdown",
    "reboot",
    "> /dev/sda",
    "chmod -R 777 /",
    "curl | sh",
    "wget | sh",
];

const SOFT_BLOCK: &[&str] = &[
    "rm ",
    "kill ",
    "drop ",
    "truncate ",
    "format ",
    "mkfs",
    "chmod ",
    "chown ",
];

pub fn read_file(repo_dir: &Path, args: &Value) -> Result<String> {
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("read_file requires path"))?;
    let max_lines = args.get("maxLines").and_then(Value::as_u64).unwrap_or(200) as usize;
    let full = repo_dir.join(path);
    let content = fs::read_to_string(&full)?;
    let mut out = String::new();
    for (idx, line) in content.lines().take(max_lines).enumerate() {
        out.push_str(&format!("{:>4}: {}\n", idx + 1, line));
    }
    if out.is_empty() {
        Ok("(empty file)".to_string())
    } else {
        Ok(out)
    }
}

pub fn cmd_execute(repo_dir: &Path, args: &Value) -> Result<String> {
    let command = args
        .get("command")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("cmd_execute requires command"))?;
    let confirmed = args
        .get("confirmed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let cwd_override = args.get("cwd").and_then(Value::as_str);
    let cwd = cwd_override
        .map(|c| repo_dir.join(c))
        .unwrap_or_else(|| repo_dir.to_path_buf());

    let lower = command.to_lowercase();
    if HARD_DENY.iter().any(|needle| lower.contains(needle)) {
        return Ok("Blocked by hard safety denylist".to_string());
    }
    if SOFT_BLOCK.iter().any(|needle| lower.contains(needle)) && !confirmed {
        return Ok(
            "Warning: destructive command detected. Re-run with confirmed=true to proceed."
                .to_string(),
        );
    }

    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        Ok(if stdout.is_empty() {
            "(ok)".to_string()
        } else {
            stdout
        })
    } else {
        let code = output.status.code().unwrap_or(-1);
        let detail = if stderr.is_empty() {
            stdout
        } else {
            stderr
        };
        Ok(format!("⚠ exit {}: {}", code, detail))
    }
}
