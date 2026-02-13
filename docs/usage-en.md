# octo-code Usage Guide

**octo-code** is an AI coding assistant CLI tool that runs in the terminal. It uses LLM (Large Language Models) to autonomously perform code writing, modification, and debugging tasks.

---

## 📦 Installation

### Prerequisites

- [Rust](https://rustup.rs/) 1.75 or higher
- Atlas Cloud or OpenRouter API key

### Install from Source

```bash
# Clone the repository
git clone https://github.com/johunsang/octo-code-agent
cd octo-code-agent

# Install
cargo install --path .

# Or release build
make install
```

---

## 🔑 Initial Setup

### 1. Configure API Key

You will be prompted to enter your API key on first run:

```bash
$ octo-code
🔑 Enter your Atlas Cloud API key (press Enter for OpenRouter): sk-...
✅ Configuration saved.
```

### 2. Manual Configuration

You can also manually create the configuration file:

**macOS:**
```bash
mkdir -p ~/Library/Application\ Support/octo-code
cat > ~/Library/Application\ Support/octo-code/config.json << 'EOF'
{
  "api_key": "sk-your-atlas-api-key",
  "api_keys": ["sk-your-atlas-api-key"],
  "openrouter_api_key": "sk-your-openrouter-key",
  "provider_type": "atlas_cloud",
  "base_url": "https://api.atlascloud.ai",
  "agent": {
    "coder_model": "zai-org/glm-5",
    "fast_model": "zai-org/glm-4.7",
    "reasoning_model": "qwen/qwen3-max-2026-01-23",
    "long_context_model": "moonshotai/kimi-k2.5",
    "max_tokens": 16384
  },
  "shell": {
    "path": "/bin/bash",
    "args": []
  },
  "context_paths": [
    "CLAUDE.md",
    "CLAUDE.local.md",
    "octo-code.md"
  ],
  "debug": false
}
EOF
```

**Linux:**
```bash
mkdir -p ~/.config/octo-code
cat > ~/.config/octo-code/config.json << 'EOF'
{
  "api_key": "sk-your-api-key",
  "provider_type": "atlas_cloud"
}
EOF
```

### 3. Environment Variables

```bash
# Using Atlas Cloud
export ATLAS_API_KEY="sk-your-api-key"

# Or using OpenRouter
export OPENROUTER_API_KEY="sk-your-api-key"

# Multiple keys for load balancing
export ATLAS_API_KEYS="key1,key2,key3"
```

---

## 🚀 Basic Usage

### Interactive Mode (Default)

Running without a prompt starts interactive mode:

```bash
$ octo-code
🐙 octo-code v0.1.0

Select a model:
1. GLM-5 (zai-org/glm-5) - $0.80/$2.56 per 1M tokens [default]
2. GLM-4.7 (zai-org/glm-4.7) - $0.52/$1.75 per 1M tokens
3. DeepSeek V3.2 (deepseek-ai/deepseek-v3.2-speciale) - $0.26/$0.38 per 1M tokens
4. Qwen3 Max (qwen/qwen3-max-2026-01-23) - $1.20/$6.00 per 1M tokens
5. Qwen3 Coder (Qwen/Qwen3-Coder) - $0.78/$3.90 per 1M tokens
6. Kimi K2.5 (moonshotai/kimi-k2.5) - $0.50/$2.50 per 1M tokens

Select (1-6, default: 1): 1

octo> Analyze the structure of this project
🔍 Exploring files...
...
```

### Single Execution Mode (-p)

Execute a specific prompt once:

```bash
octo-code -p "Fix this bug"
octo-code --prompt "Write a README.md"
```

### REPL Mode

```bash
octo-code --repl
```

### TUI Mode

Use the terminal UI for an interactive experience:

```bash
octo-code --tui
```

---

## 📋 Command Options

```
USAGE:
    octo-code [OPTIONS]

OPTIONS:
    -p, --prompt <PROMPT>     Single prompt to execute
    -c, --cwd <PATH>          Specify working directory
    -f, --output-format <FMT> Output format (text, json) [default: text]
    -q, --quiet               Suppress progress indicators
        --repl                Run in REPL mode
        --tui                 Run in TUI mode
        --session <SESSION_ID> Resume a previous session
    -m, --model <MODEL_ID>    Specify which model to use
        --provider <PROVIDER> API provider (atlas, openrouter)
    -d, --debug               Enable debug logging
    -h, --help                Display help
    -V, --version             Display version
```

---

## 💬 Chat Commands

Special commands you can use during a conversation:

| Command | Description |
|---------|-------------|
| `/quit`, `/q`, `exit` | Exit the application |
| `/help`, `/h` | Show help information |
| `/clear` | Clear the screen |
| `/sessions` | List saved sessions |
| `/session <ID>` | Load a specific session |
| `/new` | Start a new session |
| `/model` | Check current model |
| `/cost` | Check token usage and cost |

---

## 🛠️ AI Tools Usage

octo-code provides various tools that allow the AI to directly manipulate code.

### View File

```
> Show me the contents of src/main.rs
```

The AI will automatically use the `view` tool:
```
📝 view: src/main.rs
```

### Edit File

```
> Fix the bug on line 37
```

The AI will use the `edit` tool to make changes:
```
✏️ edit: src/main.rs (line 37)
```

### Create File

```
> Create a logging function in utils.rs
```

```
📝 write: src/utils.rs
```

### Execute Commands

```
> Run the tests
```

Dangerous commands require permission:
```
⚠️  Permission requested: bash { command: "cargo test" }
Allow? [y]es / [n]o / [a]lways: y
🔧 bash: cargo test
```

### Code Search

```
> Find files containing "TODO" comments
```

```
🔍 grep: TODO
```

---

## 🔐 Permission System

Some tools require user confirmation:

| Tool | Permission Required | Description |
|------|---------------------|-------------|
| `view` | ❌ None | Read files |
| `ls` | ❌ None | List directories |
| `glob` | ❌ None | File pattern search |
| `grep` | ❌ None | Code search |
| `coderlm` | ❌ None | Code intelligence |
| `task_get` | ❌ None | Get task |
| `task_list` | ❌ None | List tasks |
| `check_inbox` | ❌ None | Check inbox |
| `write` | ✅ Required | Create/write files |
| `edit` | ✅ Required | Modify files |
| `bash` | ✅ Dangerous | Execute shell commands |
| `team_create` | ✅ Required | Create teams |
| `team_delete` | ✅ Required | Delete teams |
| `spawn_agent` | ✅ Required | Spawn agents |
| `task_create` | ✅ Required | Create tasks |
| `task_update` | ✅ Required | Update tasks |
| `send_message` | ✅ Required | Send messages |

**Auto-approved commands**: `ls`, `pwd`, `echo`, `cat`, `git status`, `git log`, `git diff`, and other safe commands.

**Permission prompt example**:
```
⚠️  Permission requested: bash { command: "rm -rf target" }
Allow? [y]es / [n]o / [a]lways: 
```

---

## 💾 Session Management

### Automatic Saving

All conversations are automatically saved to a SQLite database.

**Database location:**
- macOS: `~/Library/Application Support/octo-code/octo-code.db`
- Linux: `~/.local/share/octo-code/octo-code.db`

### List Sessions

```
> /sessions
```

Example output:
```
📋 Saved sessions:
   • sess_abc123 - "Bug fix" - 2026-02-13 10:30
   • sess_def456 - "Refactoring" - 2026-02-12 15:45
```

### Resume Session

```bash
# Resume a specific session by ID
octo-code --session sess_abc123
```

Switch sessions during conversation:
```
> /session sess_abc123
```

---

## 👥 Team Collaboration (Advanced)

Run multiple AI agents in parallel to divide complex tasks.

### Create Team

The AI automatically uses the `team_create` tool:

```
> Set up a team to create a Next.js landing page
```

```
[team_create: landing-page]
[spawn_agent: layout]    ← layout + navigation
[spawn_agent: hero]      ← hero section + CTA
[spawn_agent: features]  ← feature cards + footer
```

### Task Management

Agents are coordinated via file-based task board:

```
~/.octo-code/
├── teams/{team-name}/
│   ├── config.json         # Team config
│   └── inboxes/            # Per-agent inboxes
└── tasks/{team-name}/      # Task board
```

### Delete Team

```
> Delete the landing-page team
```

---

## 🎯 Usage Examples

### Example 1: Bug Fix

```bash
$ octo-code -p "Fix the parsing error in src/parser.rs"
```

AI workflow:
1. Read file (`view`)
2. Analyze code
3. Make changes (`edit`)
4. Run tests (`bash`)

### Example 2: Add New Feature

```bash
$ octo-code
> Add user authentication middleware
```

### Example 3: Code Review

```bash
$ octo-code -p "Review the code in src/auth.rs"
```

### Example 4: Documentation

```bash
$ octo-code -p "Write API documentation to docs/api.md"
```

### Example 5: Refactoring

```bash
$ octo-code -p "Remove duplicate code and refactor"
```

### Example 6: Use Specific Model

```bash
$ octo-code -m "deepseek-ai/deepseek-v3.2-speciale" -p "Optimize this code"
```

### Example 7: Use OpenRouter

```bash
$ export OPENROUTER_API_KEY="sk-..."
$ octo-code --provider openrouter -p "Review this code"
```

---

## 💰 Pricing Information

Costs are incurred based on API usage.

| Model | Input $/1M tokens | Output $/1M tokens | Context |
|-------|-------------------|--------------------|---------|
| `zai-org/glm-5` | $0.80 | $2.56 | 202K |
| `zai-org/glm-4.7` | $0.52 | $1.75 | 202K |
| `deepseek-ai/deepseek-v3.2-speciale` | $0.26 | $0.38 | 163K |
| `qwen/qwen3-max-2026-01-23` | $1.20 | $6.00 | 252K |
| `Qwen/Qwen3-Coder` | $0.78 | $3.90 | 262K |
| `moonshotai/kimi-k2.5` | $0.50 | $2.50 | 262K |

**Cost-saving tips**:
- `-p` mode runs without a session, reducing history costs
- Use `GLM-4.7` or `DeepSeek V3.2` for small tasks
- Agent loops accumulate input tokens with each iteration
- Use `--quiet` option to monitor token usage in real-time

---

## 🔧 Troubleshooting

### API Key Error

```
Error: No API key found
```

Solution:
```bash
# Check environment variables
export ATLAS_API_KEY="sk-your-key"

# Or check config file
ls ~/Library/Application\ Support/octo-code/config.json  # macOS
ls ~/.config/octo-code/config.json                        # Linux
```

### Build Failure

```bash
# Update dependencies
cargo update

# Clean build
cargo clean && cargo build --release
```

### Database Error

```bash
# Reinitialize database
rm ~/Library/Application\ Support/octo-code/octo-code.db  # macOS
rm ~/.local/share/octo-code/octo-code.db                   # Linux
```

### Rate Limit Error

```
Rate limited. Waiting 5s... (attempt 1/3)
```

This message indicates automatic retry. You can set multiple API keys for load balancing:

```bash
export ATLAS_API_KEYS="key1,key2,key3"
```

---

## 📚 Additional Resources

- [Architecture Document (Korean)](architecture-ko.md)
- [Architecture Document (English)](architecture-en.md)
- [GitHub Issues](https://github.com/johunsang/octo-code-agent/issues)

---

## 📝 License

MIT License
