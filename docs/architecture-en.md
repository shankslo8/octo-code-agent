# octo-code-agent Architecture Analysis

## 1. Overall Structure

```
┌─────────────────────────────────────────────────────────────┐
│                    octo-code (binary)                        │
│  main.rs → clap parsing → build_app() → repl / interactive │
└──────────┬──────────────────────────┬───────────────────────┘
           │                          │
    ┌──────▼──────┐          ┌────────▼────────┐
    │  REPL mode  │          │  -p mode (once) │
    │  stdin loop │          │  auto-approve   │
    └──────┬──────┘          └────────┬────────┘
           │                          │
           └──────────┬───────────────┘
                      │
              ┌───────▼────────┐
              │     agent      │ ← core orchestrator
              │  Agent.run()   │
              └───────┬────────┘
                      │
         ┌────────────┼────────────┐
         │            │            │
  ┌──────▼──────┐ ┌───▼───┐ ┌─────▼─────┐
  │   Provider  │ │ Tools │ │ Permission│
  │(Atlas Cloud)│ │ (17)  │ │  Service  │
  │(OpenRouter) │ └───────┘ └───────────┘
  └─────────────┘
         │
  ┌──────▼──────┐
  │    core     │ ← shared type definitions
  └──────┬──────┘
         │
  ┌──────▼───────┐
  │   storage    │ ← SQLite persistence
  └──────────────┘
```

### Single Crate Structure

| Module | Path | Role | Depends On |
|--------|------|------|------------|
| `core` | `src/core/` | Types, traits, errors, config | None (bottom layer) |
| `providers` | `src/providers/` | Atlas Cloud/OpenRouter API | core |
| `tools` | `src/tools/` | Tool execution (17 tools) | core |
| `agent` | `src/agent/` | Agent loop orchestration | core |
| `storage` | `src/storage/` | SQLite DB | core |
| `cli` | `src/cli/` | Binary entry point | All of the above |

Dependencies flow in **one direction**: storage → core → providers/tools/agent → cli.
No circular dependencies, so each module can be tested independently.

---

## 2. LLM Provider Integration

### 2.1 Dual Provider Support

Supports both **Atlas Cloud** (default) and **OpenRouter**:

```
Atlas Cloud:  https://api.atlascloud.ai/api/v1/chat/completions
OpenRouter:   https://openrouter.ai/api/v1/chat/completions

Auth: Authorization: Bearer <API_KEY>
Format: OpenAI ChatCompletion compatible
```

**Advantages**:
- One Atlas Cloud API key for all models
- Same models available via OpenRouter
- Runtime switching with `--provider` flag

### 2.2 Registered Models (6)

| Model ID | Vendor | Highlights | Input $/M | Output $/M | Context |
|---------|--------|------------|-----------|------------|---------|
| `zai-org/glm-5` | Zhipu AI | Agent-optimized, multi-step reasoning | $0.80 | $2.56 | 202K |
| `zai-org/glm-4.7` | Zhipu AI | Cost-effective, fast response | $0.52 | $1.75 | 202K |
| `deepseek-ai/deepseek-v3.2-speciale` | DeepSeek | 685B MoE, lowest price | $0.26 | $0.38 | 163K |
| `qwen/qwen3-max-2026-01-23` | Alibaba | Flagship, strong reasoning | $1.20 | $6.00 | 252K |
| `Qwen/Qwen3-Coder` | Alibaba | 480B MoE, code-optimized | $0.78 | $3.90 | 262K |
| `moonshotai/kimi-k2.5` | Moonshot AI | Ultra-long context, multimodal | $0.50 | $2.50 | 262K |

**Default model**: `zai-org/glm-5` (agent-optimized)

**Budget model**: `deepseek-ai/deepseek-v3.2-speciale` (lowest price)

### 2.3 Configuration

```bash
# Environment variables
export ATLAS_API_KEY="your-key-here"
export OPENROUTER_API_KEY="your-key-here"

# Or automatic setup on first run
octo-code

# Config file (JSON format)
# macOS: ~/Library/Application Support/octo-code/config.json
# Linux: ~/.config/octo-code/config.json
```

Key detection priority: `ATLAS_API_KEY` → `ATLAS_CLOUD_API_KEY` → `OPENAI_API_KEY` → `ANTHROPIC_API_KEY`

---

## 3. Core Principle: Agent Loop

The **most important mechanism** in this project. This is why the LLM can "autonomously" modify code.

### 3.1 Basic Concept

```
User: "Fix the bug in this file"
  ↓
LLM: "Let me read the file first" + [tool_use: view {path: "main.rs"}]
  ↓
Agent: executes view tool → sends result back to LLM
  ↓
LLM: "There's an off-by-one error on line 37" + [tool_use: edit {...}]
  ↓
Agent: executes edit tool → sends result back to LLM
  ↓
LLM: "Fixed. Let me run the tests" + [tool_use: bash {command: "cargo test"}]
  ↓
Agent: executes bash tool → sends result back to LLM
  ↓
LLM: "All tests pass." [end_turn]
  ↓
Agent: loop exits
```

**Key insight**: The loop repeats until the LLM returns `end_turn`. The LLM decides which tools to call, examines results, and determines next actions on its own.

### 3.2 Code Flow (agent.rs)

```
Agent.run(session_id, messages, user_input)
  │
  ├─ tokio::spawn(agent_loop)    ← runs as separate async task
  │    │
  │    ├─ messages.push(user_msg)
  │    │
  │    └─ loop {                  ← ★ core loop
  │         │
  │         ├─ provider.stream_response(messages, system_prompt, tools)
  │         │   → HTTP POST (SSE streaming)
  │         │
  │         ├─ process_stream() → (assistant_msg, finish_reason, usage)
  │         │   → assembles Message from streaming events
  │         │
  │         ├─ messages.push(assistant_msg)
  │         │
  │         ├─ context window management (trim old messages)
  │         │
  │         └─ match finish_reason {
  │              EndTurn → return Ok(())     ← exit loop
  │              ToolUse → {
  │                for each tool_call {
  │                  tool.run(call, ctx)     ← execute tool
  │                }
  │                messages.push(tool_results)
  │                continue                  ← continue loop
   │              }
  │            }
  │
  └─ return (rx, cancel_token)   ← CLI receives events
```

### 3.3 FinishReason Semantics

| FinishReason | Meaning | Agent Behavior |
|-------------|---------|----------------|
| `EndTurn` | Nothing more to say | **Exit loop** |
| `ToolUse` | Wants to use a tool | Execute tool, **continue loop** |
| `MaxTokens` | Output limit exceeded | Exit loop |
| `Cancelled` | User cancelled | Exit loop |

**The loop only continues on ToolUse** — this is the mechanism that enables "autonomous agent behavior."

### 3.4 Rate Limit Handling

```rust
// 3 retries + exponential backoff
for attempt in 0..3 {
    match provider.stream_response(...).await {
        Ok(stream) => break,
        Err(RateLimited { retry_after_ms }) => {
            let wait = retry_after_ms * (attempt + 1);
            sleep(wait).await;
        }
    }
}
```

---

## 4. Streaming Architecture

### 4.1 Why Streaming

LLM responses take seconds to tens of seconds. Waiting for the full response results in poor UX.
→ **Output tokens immediately as they arrive** (typewriter effect like ChatGPT)

### 4.2 Three-Stage Event Transformation Pipeline

```
[LLM API]               [Provider]              [Agent]              [CLI]
 SSE bytes  ──parse──→  ProviderEvent  ──process──→  AgentEvent  ──render──→  Terminal
 (HTTP)                  (internal                 (UI events)           (stdout)
                          abstraction)
```

**Stage 1: Provider (SSE → ProviderEvent)**
```rust
// providers/openai.rs - parse SSE bytes into abstract events
match delta {
    content → yield ProviderEvent::ContentDelta { text }
    tool_calls → yield ProviderEvent::ToolUseStart { id, name }
    finish_reason → yield ProviderEvent::Complete { finish_reason, usage }
}
```

**Stage 2: Agent (ProviderEvent → AgentEvent)**
```rust
// agent/agent.rs - process_stream()
match event {
    ContentDelta { text } → {
        current_text += text;             // accumulate in message
        tx.send(AgentEvent::ContentDelta) // forward to CLI immediately
    }
    ToolUseStart → { /* tool call initiated */ }
    ToolUseStop  → { /* add ToolCall to Message.parts */ }
    Complete     → { /* store finish_reason */ }
}
```

**Stage 3: CLI (AgentEvent → Terminal Output)**
```rust
// cli/output.rs
match event {
    ContentDelta { text }     → print!("{text}")     // real-time typewriter
    ToolCallStart { name }    → eprintln!("[tool: {name}]")
    ToolResult { result, .. } → eprintln!(result)
    Complete { usage, .. }    → eprintln!("[tokens: ...]")
}
```

### 4.3 Channel-Based Async Communication

```
┌──────────┐   mpsc::channel(256)   ┌──────────┐
│  Agent   │ ─── AgentEvent ──────→ │   CLI    │
│  (task)  │                        │  (main)  │
└──────────┘                        └──────────┘
     ↑                                   │
     │  CancellationToken                │
     └──────── cancel signal ────────────┘
```

- `mpsc::channel`: Agent → CLI unidirectional event stream
- `CancellationToken`: CLI → Agent cancellation signal (Ctrl-C)
- Agent runs in a separate `tokio::spawn` task → no CLI blocking

---

## 5. Tool System

### 5.1 How LLM Tool Calling Works

LLMs can **only produce text**. They cannot read files or execute commands.
→ When "tool definitions" are provided to the LLM, it **requests tool calls via structured JSON**.

### 5.2 Tool Interface

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn definition(&self) -> ToolDefinition;    // JSON Schema (sent to LLM)
    async fn run(&self, call: &ToolCall, ctx: &ToolContext) 
        -> Result<ToolResult, ToolError>;
}
```

### 5.3 Current Tool List (17 tools)

| Category | Tool | Role | Permission Required |
|----------|------|------|---------------------|
| File | `view` | Read files | ❌ None |
| | `write` | Create/overwrite files | ✅ Required |
| | `edit` | Modify files via string replacement | ✅ Required |
| Search | `ls` | Directory listing | ❌ None |
| | `glob` | Search files by pattern | ❌ None |
| | `grep` | Search code by regex | ❌ None |
| Execute | `bash` | Shell command execution | ✅ Dangerous |
| Code | `coderlm` | CodeRLM code intelligence | ❌ None |
| Team | `team_create` | Create team | ✅ Required |
| | `team_delete` | Delete team | ✅ Required |
| | `spawn_agent` | Spawn agent | ✅ Required |
| Task | `task_create` | Create task | ✅ Required |
| | `task_get` | Get task | ❌ None |
| | `task_update` | Update task | ✅ Required |
| | `task_list` | List tasks | ❌ None |
| Message | `send_message` | Send message | ✅ Required |
| | `check_inbox` | Check inbox | ❌ None |

---

## 6. Message System

### 6.1 ContentPart (Polymorphic Messages)

```rust
pub enum ContentPart {
    Text { text: String }
    Reasoning { text: String }
    ToolCall { id: String, name: String, input: String }
    ToolResult { tool_call_id: String, content: String, is_error: bool }
    Finish { reason: FinishReason, timestamp: DateTime<Utc> }
    Image { data: String, media_type: String }
    ImageUrl { url: String }
}
```

**Tagged enum** + serde for JSON serialization. A single Message can contain multiple content types.

---

## 7. Permission System

The LLM calls tools autonomously, but **dangerous operations require user approval**.

```
Tool execution request
    ├─ Safe command? (ls, git status, echo, etc.) → auto-approve
    ├─ -p mode? → auto-approve
    └─ Otherwise → prompt user in CLI
         Allow? [y]es / [n]o / [a]lways:
```

---

## 8. Storage (SQLite)

```sql
-- Sessions table
sessions (
    id, title, message_count, 
    prompt_tokens, completion_tokens, cost,
    created_at, updated_at
)

-- Messages table
messages (
    id, session_id, role, parts_json, 
    model_id, usage_json, created_at, updated_at
)

-- File versions
files (
    id, session_id, path, content, 
    version, created_at, updated_at
)
```

WAL mode, embedded, no server required.

---

## 9. Team Collaboration System (Parallel Multi-Agent)

### 9.1 Concept

Automatically decompose complex tasks for parallel processing by multiple agents:

```
User: "Create a Next.js landing page"
    ↓
Lead Agent: Task decomposition
    ├─ spawn_agent: layout (layout + navigation)
    ├─ spawn_agent: hero (hero section + CTA)
    └─ spawn_agent: features (feature cards + footer)
    ↓
Agents work in parallel → file-based task board for coordination
    ↓
Lead Agent: Result integration and verification
```

### 9.2 File-Based Coordination

```
~/.octo-code/
├── teams/{team-name}/
│   ├── config.json         # Team config, member list
│   └── inboxes/
│       └── {agent}.json    # Per-agent message queue
└── tasks/{team-name}/
    ├── counter.json        # Task ID counter
    └── {id}.json           # Individual task
```

---

## 10. Full Sequence Diagram

```
User          CLI           Agent         Provider      LLM API       Tool
 │               │               │              │              │            │
 │──"Fix bug"───→│               │              │              │            │
 │               │──run()───────→│              │              │            │
 │               │               │──stream()───→│              │            │
 │               │               │              │──HTTP POST──→│            │
 │               │               │              │←─SSE:text────│            │
 │               │←─ContentDelta─│←─ContentDelta│              │            │
 │←─print("...")─│               │              │←─SSE:tool────│            │
 │               │               │              │←─SSE:stop────│            │
 │               │               │  [finish = ToolUse]         │            │
 │               │               │──────────────────────────────────run()──→│
 │               │               │←─────────────────────────────result─────│
 │               │               │──stream()───→│──HTTP POST──→│            │
 │               │               │              │←─SSE:text────│            │
 │               │               │              │←─SSE:stop────│            │
 │               │               │  [finish = EndTurn]         │            │
 │               │←─Complete─────│              │              │            │
 │←─[tokens:...]─│               │              │              │            │
```

---

## 11. Cost Calculation

```
Cost = (input tokens / 1M) × input price + (output tokens / 1M) × output price
```

DeepSeek V3.2 Speciale example:
```
Input  10,000 tokens × $0.26/M = $0.0026
Output  2,000 tokens × $0.38/M = $0.00076
Total                            = $0.00336
```

**Agent loop cost characteristics**: The entire conversation history is re-sent every loop iteration → input tokens accumulate.
Cost increases with more tool usage (managed by context trimming).
