# bluegit

`bluegit` is an interactive Rust CLI Git agent with real tool execution via shell and git.

## Features

- REPL with slash commands (`/auth`, `/model`, `/cd`, `/clear`, `/history`, `/tools`, `/exit`)
- OpenRouter + Inception provider support using OpenAI-compatible chat completions with tool calling
- Real agent loop with tool call execution until final assistant answer
- Built-in safety checks for secrets, dangerous push patterns, and destructive shell commands

## Setup

1. Copy environment template:

```bash
cp .env.example .env
```

2. Set your API keys in `.env`:

- `OPENROUTER_API_KEY`
- `INCEPTION_API_KEY`
- `DEFAULT_PROVIDER` (`openrouter` or `inception`)

3. Build:

```bash
cargo build
```

4. Run locally:

```bash
cargo run
```

## Install as a global CLI

Install from this folder:

```bash
cargo install --path .
```

After installation, run from any directory:

```bash
bluegit
```

If `bluegit` is not found, add Cargo's bin directory to your `PATH`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Install without Rust (GitHub Releases)

- Maintainers: create a tag like `v0.1.0` and push it to trigger binary builds.
- Users: download the matching binary from GitHub Releases and place it in a directory on `PATH` (for example `/usr/local/bin` on Linux/macOS).
- Rename if needed to `bluegit` and make it executable:

```bash
chmod +x bluegit
```

## Prompt Format

The prompt line is:

```text
[provider/model] /path/to/repo >
```

## Agent Loop

For each user message:

1. Send full conversation + all tool definitions
2. If response includes tool calls, execute tools and append tool outputs
3. Re-query model
4. Stop when plain text response arrives
5. Hard cap at 10 iterations per turn

## Tool List

- Git: status, diff, log, add, commit, push, pull, branch, checkout, stash, reset, revert, cherry-pick, show, blame, remote, fetch, merge, rebase, clean, bisect
- Shell: `read_file`, `cmd_execute`

## Safety

- `git_add` / `git_commit` block common secret files (`.env`, keys, certs, `credentials.json`, etc.)
- `git_push` requires explicit `confirmed=true` for `main/master` and force push
- `cmd_execute` hard-blocks known catastrophic commands and soft-blocks destructive commands unless `confirmed=true`
