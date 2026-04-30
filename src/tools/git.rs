use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, Result};
use serde_json::Value;

fn run_git(repo_dir: &Path, args: &[String]) -> Result<String> {
    let output = Command::new("git").args(args).current_dir(repo_dir).output()?;
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

fn is_secret_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    let file = Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path);
    lower == ".env"
        || lower.starts_with(".env.")
        || lower.ends_with(".pem")
        || lower.ends_with(".key")
        || lower.ends_with(".p12")
        || file == "id_rsa"
        || file == "credentials.json"
}

fn list_candidate_paths_for_dot(repo_dir: &Path) -> Result<Vec<String>> {
    let status = run_git(repo_dir, &["status".into(), "--porcelain".into()])?;
    Ok(status
        .lines()
        .filter_map(|line| line.get(3..).map(str::trim))
        .map(str::to_string)
        .collect())
}

fn ensure_no_secret_paths(paths: &[String]) -> Result<()> {
    let bad: Vec<String> = paths
        .iter()
        .filter(|p| is_secret_path(p))
        .cloned()
        .collect();
    if bad.is_empty() {
        Ok(())
    } else {
        Err(anyhow!(
            "Blocked by safety policy. Sensitive file(s): {}",
            bad.join(", ")
        ))
    }
}

pub fn git_status(repo_dir: &Path) -> Result<String> {
    run_git(repo_dir, &["status".into(), "--porcelain".into()])
}

pub fn git_diff(repo_dir: &Path, args: &Value) -> Result<String> {
    let staged = args.get("staged").and_then(Value::as_bool).unwrap_or(false);
    if staged {
        run_git(repo_dir, &["diff".into(), "--staged".into()])
    } else {
        run_git(repo_dir, &["diff".into(), "HEAD".into()])
    }
}

pub fn git_log(repo_dir: &Path, args: &Value) -> Result<String> {
    let n = args.get("n").and_then(Value::as_u64).unwrap_or(20);
    run_git(
        repo_dir,
        &[
            "log".into(),
            "--oneline".into(),
            "--graph".into(),
            "--decorate".into(),
            "-n".into(),
            n.to_string(),
        ],
    )
}

pub fn git_add(repo_dir: &Path, args: &Value) -> Result<String> {
    let files = args
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("git_add requires files: [..]"))?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if files.is_empty() {
        return Err(anyhow!("git_add requires at least one file"));
    }

    let mut to_check = files.clone();
    if files.iter().any(|f| f == ".") {
        to_check.extend(list_candidate_paths_for_dot(repo_dir)?);
    }
    ensure_no_secret_paths(&to_check)?;

    let mut args = vec!["add".to_string()];
    args.extend(files);
    run_git(repo_dir, &args)
}

pub fn git_commit(repo_dir: &Path, args: &Value) -> Result<String> {
    let message = args
        .get("message")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("git_commit requires message"))?;

    let staged = run_git(
        repo_dir,
        &["diff".into(), "--cached".into(), "--name-only".into()],
    )?;
    let paths: Vec<String> = staged.lines().map(str::to_string).collect();
    ensure_no_secret_paths(&paths)?;

    run_git(repo_dir, &["commit".into(), "-m".into(), message.to_string()])
}

pub fn git_push(repo_dir: &Path, args: &Value) -> Result<String> {
    let branch = args
        .get("branch")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("git_push requires branch"))?;
    let force = args.get("force").and_then(Value::as_bool).unwrap_or(false);
    let confirmed = args
        .get("confirmed")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if (branch == "main" || branch == "master") && !confirmed {
        return Ok("Blocked: pushing to main/master requires confirmed=true".to_string());
    }
    if force && !confirmed {
        return Ok("Blocked: force push requires confirmed=true".to_string());
    }

    let mut push_args = vec!["push".to_string()];
    if force {
        push_args.push("--force-with-lease".to_string());
    }
    push_args.push("origin".to_string());
    push_args.push(branch.to_string());
    run_git(repo_dir, &push_args)
}

pub fn git_pull(repo_dir: &Path, args: &Value) -> Result<String> {
    let rebase = args.get("rebase").and_then(Value::as_bool).unwrap_or(false);
    if rebase {
        run_git(repo_dir, &["pull".into(), "--rebase".into()])
    } else {
        run_git(repo_dir, &["pull".into()])
    }
}

pub fn git_branch(repo_dir: &Path, args: &Value) -> Result<String> {
    let action = args.get("action").and_then(Value::as_str).unwrap_or("list");
    match action {
        "list" => run_git(repo_dir, &["branch".into(), "-a".into()]),
        "create" => {
            let name = args
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("git_branch create requires name"))?;
            run_git(repo_dir, &["branch".into(), name.to_string()])
        }
        "delete" => {
            let name = args
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("git_branch delete requires name"))?;
            run_git(repo_dir, &["branch".into(), "-d".into(), name.to_string()])
        }
        "rename" => {
            let old = args
                .get("old")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("git_branch rename requires old"))?;
            let new = args
                .get("new")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("git_branch rename requires new"))?;
            run_git(
                repo_dir,
                &["branch".into(), "-m".into(), old.to_string(), new.to_string()],
            )
        }
        _ => Err(anyhow!("Unknown git_branch action")),
    }
}

pub fn git_checkout(repo_dir: &Path, args: &Value) -> Result<String> {
    let target = args
        .get("target")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("git_checkout requires target"))?;
    let create = args.get("create").and_then(Value::as_bool).unwrap_or(false);
    if create {
        run_git(repo_dir, &["checkout".into(), "-b".into(), target.to_string()])
    } else {
        run_git(repo_dir, &["checkout".into(), target.to_string()])
    }
}

pub fn git_stash(repo_dir: &Path, args: &Value) -> Result<String> {
    let action = args.get("action").and_then(Value::as_str).unwrap_or("list");
    match action {
        "push" => {
            let message = args
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("blue stash");
            run_git(
                repo_dir,
                &["stash".into(), "push".into(), "-m".into(), message.to_string()],
            )
        }
        "pop" => run_git(repo_dir, &["stash".into(), "pop".into()]),
        "list" => run_git(repo_dir, &["stash".into(), "list".into()]),
        "drop" => run_git(repo_dir, &["stash".into(), "drop".into()]),
        _ => Err(anyhow!("Unknown git_stash action")),
    }
}

pub fn git_reset(repo_dir: &Path, args: &Value) -> Result<String> {
    let mode = args.get("mode").and_then(Value::as_str).unwrap_or("mixed");
    let reference = args.get("ref").and_then(Value::as_str).unwrap_or("HEAD");
    run_git(
        repo_dir,
        &[
            "reset".into(),
            format!("--{}", mode),
            reference.to_string(),
        ],
    )
}

pub fn git_revert(repo_dir: &Path, args: &Value) -> Result<String> {
    let hash = args
        .get("hash")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("git_revert requires hash"))?;
    run_git(
        repo_dir,
        &[
            "revert".into(),
            hash.to_string(),
            "--no-edit".into(),
        ],
    )
}

pub fn git_cherry_pick(repo_dir: &Path, args: &Value) -> Result<String> {
    let hash = args
        .get("hash")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("git_cherry_pick requires hash"))?;
    run_git(repo_dir, &["cherry-pick".into(), hash.to_string()])
}

pub fn git_show(repo_dir: &Path, args: &Value) -> Result<String> {
    let reference = args.get("ref").and_then(Value::as_str).unwrap_or("HEAD");
    run_git(
        repo_dir,
        &["show".into(), "--stat".into(), reference.to_string()],
    )
}

pub fn git_blame(repo_dir: &Path, args: &Value) -> Result<String> {
    let file = args
        .get("file")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("git_blame requires file"))?;
    if let Some(lines) = args.get("lines").and_then(Value::as_str) {
        run_git(
            repo_dir,
            &[
                "blame".into(),
                "-L".into(),
                lines.to_string(),
                file.to_string(),
            ],
        )
    } else {
        run_git(repo_dir, &["blame".into(), file.to_string()])
    }
}

pub fn git_remote(repo_dir: &Path) -> Result<String> {
    run_git(repo_dir, &["remote".into(), "-v".into()])
}

pub fn git_fetch(repo_dir: &Path, args: &Value) -> Result<String> {
    let prune = args.get("prune").and_then(Value::as_bool).unwrap_or(false);
    if prune {
        run_git(repo_dir, &["fetch".into(), "--prune".into()])
    } else {
        run_git(repo_dir, &["fetch".into()])
    }
}

pub fn git_merge(repo_dir: &Path, args: &Value) -> Result<String> {
    let branch = args
        .get("branch")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("git_merge requires branch"))?;
    let no_ff = args.get("no_ff").and_then(Value::as_bool).unwrap_or(false);
    if no_ff {
        run_git(
            repo_dir,
            &["merge".into(), "--no-ff".into(), branch.to_string()],
        )
    } else {
        run_git(repo_dir, &["merge".into(), branch.to_string()])
    }
}

pub fn git_rebase(repo_dir: &Path, args: &Value) -> Result<String> {
    let base = args
        .get("base")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("git_rebase requires base"))?;
    let interactive = args
        .get("interactive")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if interactive {
        return Ok(
            "Interactive rebase is blocked in this agent. Run it manually in your shell."
                .to_string(),
        );
    }
    run_git(repo_dir, &["rebase".into(), base.to_string()])
}

pub fn git_clean(repo_dir: &Path, args: &Value) -> Result<String> {
    let dry_run = args.get("dry_run").and_then(Value::as_bool).unwrap_or(true);
    if dry_run {
        run_git(repo_dir, &["clean".into(), "-n".into()])
    } else {
        run_git(repo_dir, &["clean".into(), "-fd".into()])
    }
}

pub fn git_bisect(repo_dir: &Path, args: &Value) -> Result<String> {
    let action = args
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("git_bisect requires action"))?;
    let mut cmd = vec!["bisect".to_string(), action.to_string()];
    if let Some(hash) = args.get("hash").and_then(Value::as_str) {
        cmd.push(hash.to_string());
    }
    run_git(repo_dir, &cmd)
}
