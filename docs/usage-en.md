# octo-code Usage Guide

**octo-code** is an AI coding assistant CLI tool that runs in the terminal. It uses LLM (Large Language Models) to autonomously perform code writing, modification, and debugging tasks.

---

## 📦 Installation

### Prerequisites

- [Rust](https://rustup.rs/) 1.70 or higher
- Atlas Cloud API key

### Install from Source

```bash
# Clone the repository
git clone <repository-url>
cd octo-code-agent

# Install
make install
# or
cargo install --path crates/octo-cli
```

### Automated Installation Script

```bash
curl -fsSL https://example.com/install.sh | sh
```

---

## 🔑 Initial Setup

### 1. Configure API Key

You will be prompted to enter your API key on first run:

```bash
$ octo-code
🔑 Enter your Atlas Cloud API key: sk-...
✅ Configuration saved to ~/.config/octo-code/config.toml
```

### 2. Manual Configuration

You can also manually create the configuration file at `~/.config/octo-code/config.toml`:

```toml
[atlas]
api_key = "sk-your-api-key-here"

# Optional: Default model settings
[models]
default = "deepseek-ai/deepseek-v3.2-speciale"
coder = "zai-org/glm-5"
reasoning = "qwen/qwen3-max-2026-01-23"
```

---

## 🚀 Basic Usage

### Interactive Mode (Default)

Running without a prompt starts interactive mode:

```bash
$ octo-code
🐙 octo-code v0.1.0
💬 Enter your question (quit: exit, /help: help)

> Analyze the structure of this project
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
    --repl                    Run in REPL mode
    --tui                     Run in TUI mode
    --session <SESSION_ID>    Resume a previous session
    --model <MODEL_ID>        Specify which model to use
    -h, --help                Display help
    -V, --version             Display version
```

---

## 💬 Chat Commands

Special commands you can use during a conversation:

| Command | Description |
|---------|-------------|
| `/quit`, `/q` | Exit the application |
| `/help`, `/h` | Show help information |
| `/clear` | Clear the screen |
| `/sessions` | List saved sessions |
| `/session <ID>` | Load a specific session |
| `/new` | Start a new session |

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
| `write` | ✅ Required | Create/write files |
| `edit` | ✅ Required | Modify files |
| `bash` | ✅ Dangerous | Execute shell commands |
| `team_create` | ✅ Required | Create teams |
| `spawn_agent` | ✅ Required | Spawn agents |

**Auto-approved commands**: `ls`, `pwd`, `echo`, `cat`, `git status`, `git log`, and other safe commands.

**Permission prompt example**:
```
⚠️  Permission requested: bash { command: "rm -rf target" }
Allow? [y]es / [n]o / [a]lways: 
```

---

## 💾 Session Management

### Automatic Saving

All conversations are automatically saved to a SQLite database.

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

```
> @team octo-code feature-x-team "Implement new feature"
```

### Assign Tasks

```
> @task octo-code feature-x-team "Design database schema"
> @task octo-code feature-x-team "Implement API endpoints"
> @task octo-code feature-x-team "Write unit tests"
```

### Check Task Status

```
> @list octo-code feature-x-team
```

### Delete Team

```
> @delete octo-code feature-x-team
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

---

## 💰 Pricing Information

Billed through Atlas Cloud.

| Model | Input $/1M tokens | Output $/1M tokens |
|-------|-------------------|--------------------|
| `deepseek-ai/deepseek-v3.2-speciale` | $0.27 | $0.41 |
| `zai-org/glm-5` | $0.80 | $2.56 |
| `moonshotai/kimi-k2.5` | $0.50 | $2.50 |
| `qwen/qwen3-max-2026-01-23` | $1.20 | $6.00 |

**Cost-saving tips**:
- `-p` mode runs without a session, reducing history costs
- Use `Fast` models for small tasks
- Agent loops accumulate input tokens with each iteration

---

## 🔧 Troubleshooting

### API Key Error

```
Error: Atlas API key not found
```

Solution: Check the `~/.config/octo-code/config.toml` file.

### Build Failure

```bash
# Update dependencies
cargo update

# Clean build
make clean && make build
```

### Database Error

```bash
# Reinitialize database
rm ~/.local/share/octo-code/octo-code.db
```

---

## 📚 Additional Resources

- [Architecture Document (Korean)](architecture-ko.md)
- [Architecture Document (English)](architecture-en.md)
- [GitHub Issues](https://github.com/your-repo/octo-code-agent/issues)

---

## 📝 License

MIT License
