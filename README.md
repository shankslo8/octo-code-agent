# Octo Code Agent 🐙

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Crates.io](https://img.shields.io/crates/v/octo-code-agent)](https://crates.io/crates/octo-code-agent)

A Rust-based AI coding assistant with parallel multi-agent coordination. Octo Code Agent autonomously performs coding tasks by orchestrating multiple AI agents working simultaneously on different parts of your codebase.

## ✨ Features

- **🧠 Parallel Agent Execution**: Coordinate multiple agents working simultaneously on different files
- **🔧 Full Tool Suite**: File editing, bash commands, Git operations, code analysis
- **📁 Context-Aware**: Reads your project structure and understands the codebase
- **💬 Multiple Modes**: Interactive chat, single-command, and REPL modes
- **🛡️ Permission System**: Safe execution with user approval for potentially dangerous operations
- **💾 Session Persistence**: SQLite storage for conversation history and costs
- **📊 Cost Tracking**: Real-time token usage and cost monitoring
- **🎨 TUI Interface**: Terminal user interface with interactive dialogs

## 🚀 Quick Start

### Installation

```bash
# Install from crates.io
cargo install octo-code-agent

# Or build from source
git clone https://github.com/YOUR_USERNAME/octo-code-agent
cd octo-code-agent
cargo install --path .

# Initialize (requires Atlas Cloud API key)
octo-code --setup
```

### Usage

```bash
# Interactive mode (default)
octo-code

# Single command mode
octo-code -p "Fix the bug in main.rs"

# REPL mode
octo-code --repl

# TUI mode
octo-code --tui

# Resume a session
octo-code --session <session_id>
```

## 📦 Architecture

Octo Code Agent is structured as a Cargo workspace with 6 crates:

```
octo-code-agent/
├── octo-core/          # Core types and traits
├── octo-providers/     # LLM API providers (Atlas Cloud)
├── octo-tools/         # Tool implementations
├── octo-agent/         # Agent orchestrator
├── octo-storage/       # SQLite persistence
└── octo-cli/           # CLI binary
```

### Dependencies

- **Async Runtime**: Tokio
- **HTTP Client**: Reqwest
- **Database**: SQLx (SQLite)
- **CLI Parsing**: Clap
- **TUI**: Ratatui
- **Serialization**: Serde

## 🔄 How It Works

1. **User Request**: You describe a coding task in natural language
2. **Agent Coordination**: The main agent spawns sub-agents for parallel work
3. **Tool Execution**: Agents autonomously use tools (read files, run tests, edit code)
4. **Iterative Refinement**: Agents coordinate results and refine solutions
5. **Completion**: Final solution presented with execution results

### Example Flow

```
User: "Add error handling to the authentication module"
  ↓
Agent 1: Analyzes authentication.rs, identifies error-prone sections
Agent 2: Creates error types in errors.rs
Agent 3: Updates function signatures with Result returns
Agent 4: Writes tests for error cases
  ↓
Coordinated result: Complete error handling implementation
```

## 🛠️ Tools Available

| Tool | Description | Permission Required |
|------|-------------|-------------------|
| `bash` | Execute shell commands | ⚠️ Dangerous commands |
| `view` | Read file contents | No |
| `write` | Create new files | Yes |
| `edit` | Edit existing files | Yes |
| `ls` | List directory contents | No |
| `glob` | Find files by pattern | No |
| `grep` | Search code with regex | No |
| `coderlm` | Code intelligence | No |
| `team_*` | Multi-agent coordination | Yes |
| `task_*` | Task management | Yes |

## 🔐 Safety & Permissions

Octo Code Agent uses a permission system to ensure safe execution:

- **Automatic approval**: Safe commands (ls, git status, etc.)
- **Manual approval**: Dangerous commands (rm, write system files, etc.)
- **Batch mode**: Use `-p` flag for fully automatic execution (use with caution)

## 💰 Cost Management

Uses [Atlas Cloud](https://atlas.nomic.ai/) for LLM access with transparent pricing:

| Model | Input ($/M) | Output ($/M) | Purpose |
|-------|-------------|--------------|---------|
| `deepseek-ai/deepseek-v3.2-special` | $0.27 | $0.41 | Default, cost-efficient |
| `zai-org/glm-5` | $0.80 | $2.56 | Agent-optimized |
| `moonshotai/kimi-k2.5` | $0.50 | $2.50 | Long context |
| `qwen/qwen3-max-2026-01-23` | $1.20 | $6.00 | Flagship |

Cost = (input_tokens / 1M × input_price) + (output_tokens / 1M × output_price)

## 🧪 Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- -p "Add logging to the project"

# Build release binary
cargo build --release
```

### Adding New Tools

1. Implement the `Tool` trait in `crates/octo-tools/src/`
2. Register the tool in `crates/octo-tools/src/lib.rs`
3. Add to the tools registry

## 🤝 Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

## 📚 Documentation

- [API Documentation](docs/api.md)
- [Architecture Overview](docs/architecture.md)
- [Tool Development Guide](docs/tool-development.md)
- [Provider Integration](docs/providers.md)

## 🙏 Acknowledgments

- Built with ❤️ in Rust
- Powered by [Atlas Cloud](https://atlas.nomic.ai/)
- Inspired by modern AI coding assistants

---

**Happy coding with Octo!** 🐙