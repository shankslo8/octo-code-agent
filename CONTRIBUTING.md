# Contributing to Octo Code Agent

Thank you for your interest in contributing to Octo Code Agent! This document provides guidelines and instructions for contributing.

## 🚀 Getting Started

### Prerequisites

- Rust 1.70 or higher
- Git
- An [Atlas Cloud](https://atlas.nomic.ai/) API key (for testing)

### Setting Up Development Environment

```bash
# Clone the repository
git clone https://github.com/YOUR_USERNAME/octo-code-agent
cd octo-code-agent

# Build the project
cargo build

# Run tests
cargo test

# Set up your API key
echo 'ATLAS_API_KEY="your-api-key-here"' > .env
```

## 📁 Project Structure

```
octo-code-agent/
├── crates/
│   ├── octo-core/          # Core types and traits
│   ├── octo-providers/     # LLM API providers
│   ├── octo-tools/         # Tool implementations
│   ├── octo-agent/         # Agent orchestrator
│   ├── octo-storage/       # SQLite persistence
│   └── octo-cli/          # CLI binary
├── docs/                   # Documentation
└── playground/            # Testing playground
```

## 🔧 Development Workflow

### 1. Finding Issues

Check the [Issues](https://github.com/YOUR_USERNAME/octo-code-agent/issues) page for tasks:
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

# Run specific crate tests
cargo test -p octo-tools

# Run with verbose output
cargo test -- --nocapture

# Test with logging
RUST_LOG=debug cargo test
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
// crates/octo-tools/src/your_tool.rs
use octo_core::tool::{Tool, ToolDefinition, ToolCall, ToolContext, ToolResult};

pub struct YourTool;

#[async_trait::async_trait]
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

    async fn run(&self, call: ToolCall, ctx: ToolContext) -> Result<ToolResult, ToolError> {
        // Implementation here
        Ok(ToolResult::Success {
            content: serde_json::json!({
                "result": "Success!"
            }),
        })
    }
}
```

### 2. Register the Tool

Add your tool to the tools registry in `crates/octo-tools/src/lib.rs`:

```rust
mod your_tool;

// In create_tools() function
tools.push(Arc::new(your_tool::YourTool));
```

### 3. Add Tests

Write tests for your new tool in `crates/octo-tools/src/tests.rs` or a separate test file.

## 🔌 Adding New Providers

To add support for a new LLM provider:

### 1. Create Provider Implementation

```rust
// crates/octo-providers/src/your_provider.rs
use octo_core::provider::{Provider, ProviderStream, ProviderError};

pub struct YourProvider {
    api_key: String,
    base_url: String,
}

#[async_trait::async_trait]
impl Provider for YourProvider {
    async fn stream_response(
        &self,
        messages: Vec<Message>,
        model_id: &ModelId,
        tools: Option<&[ToolDefinition]>,
        max_tokens: Option<u32>,
    ) -> Result<ProviderStream, ProviderError> {
        // Implementation here
    }
}
```

### 2. Register Provider Factory

Add to `crates/octo-providers/src/lib.rs`:

```rust
mod your_provider;

// In create_provider() or similar factory function
match model_id.provider() {
    "your_provider" => Ok(Arc::new(your_provider::YourProvider::new(config))),
    // ... other providers
}
```

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