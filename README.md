# bluegit

`bluegit` is an interactive Rust CLI Git agent with real tool execution via shell and git.

## Features

- REPL with slash commands (`/auth`, `/model`, `/cd`, `/clear`, `/history`, `/tools`, `/exit`)
- OpenRouter + Inception + **智谱 BigModel** + **Mistral** ([chat completions API](https://docs.mistral.ai/api)) — OpenAI-compatible with tool calling
- Real agent loop with tool call execution until final assistant answer
- Built-in safety checks for secrets, dangerous push patterns, and destructive shell commands

## Setup

1. Copy environment template:

```bash
cp .env.example .env
```

2. Set API keys in `.env` (at least one provider):

- `OPENROUTER_API_KEY` — optional if you use other providers
- `INCEPTION_API_KEY` — optional
- `BIGMODEL_API_KEY` or `ZHIPU_API_KEY` — [智谱 AI Open Platform](https://docs.bigmodel.cn/cn/api/introduction#curl) (Bearer `https://open.bigmodel.cn/api/paas/v4/chat/completions`)
- `MISTRAL_API_KEY` — [Mistral AI](https://docs.mistral.ai/api) (`https://api.mistral.ai/v1/chat/completions`)
- `DEFAULT_PROVIDER` — `openrouter`, `inception`, `bigmodel`, `zhipu`, or `mistral` (must match a key you set)

Optional BigModel tuning:

- `BIGMODEL_USE_CODING_ENDPOINT=true` — use the [GLM Coding endpoint](https://docs.bigmodel.cn/cn/api/introduction) (`.../api/coding/paas/v4/chat/completions`) instead of general paas v4
- `BIGMODEL_CHAT_COMPLETIONS_URL` — override the full chat-completions URL

Optional Mistral:

- `MISTRAL_CHAT_COMPLETIONS_URL` — override chat-completions URL (defaults to `https://api.mistral.ai/v1/chat/completions`)

### OpenRouter: pick a model that supports tools

`bluegit` always sends **function/tool definitions** to the API. On OpenRouter, if the model has no provider that supports tools, you get HTTP **404** with a message about `tool_choice` / endpoints — that is a **model routing** limitation, not a bad API key.

Use a model that supports the `tools` parameter. Filter on OpenRouter: [models with `tools` support](https://openrouter.ai/models?supported_parameters=tools). Many free or reasoning-only models do **not** expose tool calling; use something like `anthropic/claude-sonnet-4.5` or `openai/gpt-4o`, or set `/model` accordingly after launch.

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
