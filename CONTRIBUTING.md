# Contributing to Octo Code Agent

Thank you for your interest in contributing to Octo Code Agent! This document provides guidelines and instructions for contributing.

## 🚀 Getting Started

### Prerequisites

- Rust 1.75 or higher
- Git
- An [Atlas Cloud](https://atlascloud.ai/) or [OpenRouter](https://openrouter.ai/) API key (for testing)

### Setting Up Development Environment

```bash
# Clone the repository
git clone https://github.com/johunsang/octo-code-agent
cd octo-code-agent

# Build the project
cargo build

# Run tests
cargo test

# Set up your API key
export ATLAS_API_KEY="your-api-key-here"
# or
export OPENROUTER_API_KEY="your-api-key-here"
```

## 📁 Project Structure

```
octo-code-agent/
├── Cargo.toml              # Single crate configuration (bin + lib)
├── src/
│   ├── main.rs             # Binary entry point
│   ├── lib.rs              # Library root
│   ├── core/               # Core types and traits
│   │   ├── config.rs       # Configuration management
│   │   ├── model.rs        # Model definitions and pricing
│   │   ├── message.rs      # Message system
│   │   ├── tool.rs         # Tool trait
│   │   ├── provider.rs     # Provider trait
│   │   └── ...
│   ├── providers/          # LLM API providers
│   │   └── openai.rs       # OpenAI-compatible API (Atlas Cloud, OpenRouter)
│   ├── tools/              # Tool implementations (17 tools)
│   │   ├── bash.rs
│   │   ├── view.rs
│   │   ├── write.rs
│   │   ├── edit.rs
│   │   ├── ls.rs
│   │   ├── glob_tool.rs
│   │   ├── grep.rs
│   │   ├── coderlm.rs
│   │   ├── team.rs
│   │   ├── task_mgmt.rs
│   │   └── send_message.rs
│   ├── agent/              # Agent loop implementation
│   │   ├── agent.rs
│   │   ├── event.rs
│   │   └── prompt.rs
│   ├── storage/            # SQLite persistence
│   │   ├── database.rs
│   │   ├── session_repo.rs
│   │   └── message_repo.rs
│   └── cli/                # CLI interface
│       ├── interactive.rs
│       ├── repl.rs
│       ├── tui/            # ratatui-based terminal UI
│       └── ...
├── migrations/             # SQLite migrations
├── docs/                   # Documentation
└── playground/             # Testing playground
```

## 🔧 Development Workflow

### 1. Finding Issues

Check the [Issues](https://github.com/johunsang/octo-code-agent/issues) page for tasks:
- `good first issue` - Good for newcomers
- `bug` - Issues to fix
- `enhancement` - New features to add
- `documentation` - Documentation improvements

### 2. Creating a Branch

```bash
# Create a new branch
git checkout -b feature/your-feature-name
# or
git checkout -b fix/issue-description
```

### 3. Making Changes

Follow the existing code style:

- **Formatting**: Use `cargo fmt`
- **Linting**: Use `cargo clippy`
- **Documentation**: Add doc comments for public API
- **Tests**: Write tests for new functionality

### 4. Testing

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Test with logging
RUST_LOG=debug cargo test

# Run clippy
cargo clippy

# Check formatting
cargo fmt --check
```

### 5. Committing Changes

Use descriptive commit messages:

```
feat: add new tool for database queries
fix: resolve crash when file doesn't exist
docs: update installation instructions
refactor: simplify agent coordination logic
test: add tests for permission system
```

### 6. Creating a Pull Request

1. Push your branch to GitHub
2. Create a pull request
3. Fill out the PR template
4. Request review from maintainers

## 🧩 Adding New Tools

Tools are the building blocks of Octo Code Agent. To add a new tool:

### 1. Create the Tool Implementation

```rust
// src/tools/your_tool.rs
use crate::core::tool::{Tool, ToolDefinition, ToolCall, ToolContext, ToolResult, ToolError};
use async_trait::async_trait;

pub struct YourTool;

#[async_trait]
impl Tool for YourTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "your_tool".to_string(),
            description: "Description of what your tool does".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "param1": {
                        "type": "string",
                        "description": "Description of param1"
                    }
                },
                "required": ["param1"]
            }),
        }
    }

    async fn run(&self, call: &ToolCall, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        // Parse input
        let input: serde_json::Value = serde_json::from_str(&call.input)?;
        let param1 = input["param1"].as_str().ok_or_else(|| {
            ToolError::InvalidInput("param1 is required".to_string())
        })?;
        
        // Implementation here
        Ok(ToolResult::success("Success!".to_string()))
    }
}
```

### 2. Register the Tool

Add your tool to the tools registry in `src/tools/mod.rs`:

```rust
mod your_tool;
pub use your_tool::YourTool;

// In create_all_tools() function
tools.push(Arc::new(YourTool));
```

### 3. Add Tests

Write tests for your new tool in `src/tools/tests.rs`:

```rust
#[tokio::test]
async fn test_your_tool() {
    let tool = YourTool;
    let call = ToolCall {
        id: "test-1".to_string(),
        name: "your_tool".to_string(),
        input: r#"{"param1": "test"}"#.to_string(),
    };
    let ctx = ToolContext::default();
    
    let result = tool.run(&call, &ctx).await.unwrap();
    assert!(!result.is_error);
}
```

## 🔌 Adding New Providers

The project uses OpenAI-compatible API format. To add a new provider:

### 1. Update Model Definitions

Add your model to `src/core/model.rs`:

```rust
m.insert(
    ModelId("your-vendor/model-name".into()),
    Model {
        id: ModelId("your-vendor/model-name".into()),
        vendor: ModelVendor::YourVendor,
        display_name: "Your Model".into(),
        context_window: 131_072,
        max_output_tokens: 32_768,
        capabilities: ModelCapabilities {
            supports_tool_use: true,
            supports_streaming: true,
            supports_thinking: true,
            supports_images: false,
        },
        pricing: ModelPricing {
            cost_per_1m_input: 0.50,
            cost_per_1m_output: 1.50,
            cost_per_1m_input_cached: None,
        },
    },
);
```

### 2. Update Provider Factory (if needed)

Modify `src/providers/openai.rs` if the provider requires special handling.

## 📝 Documentation

### Code Documentation

- Add doc comments for all public APIs
- Use examples in doc comments when helpful
- Document error conditions

### User Documentation

- Update README.md for user-facing changes
- Add to docs/ directory for detailed guides
- Include examples of usage

## 🧪 Testing Guidelines

### Unit Tests

- Test individual functions and methods
- Mock external dependencies
- Test error cases

### Integration Tests

- Test tool interactions
- Test agent coordination
- Test end-to-end workflows

### Example Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_your_feature() {
        // Arrange
        let input = "test";
        
        // Act
        let result = your_function(input);
        
        // Assert
        assert_eq!(result, "expected");
    }

    #[tokio::test]
    async fn test_async_feature() {
        // Async test
    }
}
```

## 🐛 Bug Reports

When reporting bugs, please include:

1. **Description**: What happened vs what you expected
2. **Steps to Reproduce**: Clear, step-by-step instructions
3. **Environment**: Rust version, OS, octo-code-agent version
4. **Logs**: Any error messages or logs (run with `RUST_LOG=debug`)
5. **Code Example**: If applicable, code that triggers the bug

## 💬 Code Review Process

1. **Automated Checks**: CI runs tests, formatting, and linting
2. **Manual Review**: Maintainers review code for:
   - Correctness
   - Performance
   - Security
   - Maintainability
   - Documentation
3. **Feedback**: Reviewers provide constructive feedback
4. **Iteration**: Address feedback and update the PR
5. **Merge**: Once approved, the PR is merged

## 📜 Code of Conduct

All contributors are expected to follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## 🙏 Thank You!

Thank you for contributing to Octo Code Agent! Your contributions help make this tool better for everyone.

---

Questions? Feel free to open an issue or join our discussions!
