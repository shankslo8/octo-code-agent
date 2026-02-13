# octo-code-agent Architecture Analysis

## 1. Overall Structure

```
┌─────────────────────────────────────────────────────────────┐
│                      octo-cli (binary)                       │
│  main.rs → clap parsing → build_app() → repl / noninteractive│
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
              │   octo-agent   │ ← core orchestrator
              │   Agent.run()  │
              └───────┬────────┘
                      │
         ┌────────────┼────────────┐
         │            │            │
  ┌──────▼──────┐ ┌───▼───┐ ┌─────▼─────┐
  │  Provider   │ │ Tools │ │ Permission│
  │(Atlas Cloud)│ │  (7)  │ │  Service  │
  └─────────────┘ └───────┘ └───────────┘
         │
  ┌──────▼──────┐
  │  octo-core  │ ← shared type definitions
  └─────────────┘
         │
  ┌──────▼───────┐
  │ octo-storage │ ← SQLite persistence
  └──────────────┘
```

### Cargo Workspace Structure (6 crates)

| crate | Role | Depends On |
|-------|------|------------|
| `octo-core` | Types, traits, errors, config | None (bottom layer) |
| `octo-providers` | Atlas Cloud API communication | octo-core |
| `octo-tools` | Tool execution (bash, edit, etc.) | octo-core |
| `octo-agent` | Agent loop orchestration | octo-core |
| `octo-storage` | SQLite DB | octo-core |
| `octo-cli` | Binary entry point | All of the above |

Dependencies flow in **one direction**: core ← providers/tools/agent/storage ← cli.
No circular dependencies, so each crate can be tested independently.

---

## 2. Atlas Cloud Unified API

### 2.1 Single Key, Single Endpoint

All LLM calls are routed through the **Atlas Cloud** gateway:

```
Endpoint: https://api.atlascloud.ai/api/v1/chat/completions
Auth: Authorization: Bearer <ATLAS_API_KEY>
Format: OpenAI ChatCompletion compatible
```

**Advantage**: One API key for all models. No separate Provider implementations needed.

### 2.2 Registered Models (5)

| Model ID | Vendor | Highlights | Input $/M | Output $/M |
|---------|--------|------------|-----------|------------|
| `zai-org/glm-5` | Zhipu AI | Agent-optimized, multi-step reasoning | $0.80 | $2.56 |
| `moonshotai/kimi-k2.5` | Moonshot AI | Ultra-long context, native multimodality | $0.50 | $2.50 |
| `qwen/qwen3-max-2026-01-23` | Alibaba | Flagship, code generation | $1.20 | $6.00 |
| `minimaxai/minimax-m2.1` | MiniMax | 230B MoE, SWE-bench 74% | $0.30 | $0.30 |
| `deepseek-ai/deepseek-v3.2-speciale` | DeepSeek | 685B MoE, cheapest, IOI gold medal | $0.27 | $0.41 |

**Default model**: `deepseek-ai/deepseek-v3.2-speciale` (cheapest with excellent performance)

### 2.3 Configuration

```
Env var: ATLAS_API_KEY=your-key-here
Or config file:
{
  "api_key": "your-key-here",
  "base_url": "https://api.atlascloud.ai/api"
}
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
  │         │   → HTTP POST to Atlas Cloud (SSE streaming)
  │         │
  │         ├─ process_stream() → (assistant_msg, finish_reason, usage)
  │         │   → assembles Message object from streaming events
  │         │
  │         ├─ messages.push(assistant_msg)
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

---

## 4. Streaming Architecture

### 4.1 Why Streaming

LLM responses take seconds to tens of seconds. Waiting for the full response results in poor UX.
→ **Output tokens immediately as they arrive** (typewriter effect like ChatGPT)

### 4.2 Three-Stage Event Transformation Pipeline

```
[Atlas Cloud]           [Provider]              [Agent]              [CLI]
 SSE bytes  ──parse──→  ProviderEvent  ──process──→  AgentEvent  ──render──→  Terminal
 (HTTP)                  (internal      (UI events)           (stdout)
                          abstraction)
```

**Stage 1: Provider (SSE → ProviderEvent)**
```rust
// openai.rs - parse SSE bytes into abstract events
match delta {
    content → yield ProviderEvent::ContentDelta { text }
    tool_calls → yield ProviderEvent::ToolUseStart { id, name }
    finish_reason → yield ProviderEvent::Complete { finish_reason, usage }
}
```

**Stage 2: Agent (ProviderEvent → AgentEvent)**
```rust
// agent.rs - process_stream()
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
// output.rs
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
trait Tool: Send + Sync {
    fn definition(&self) -> ToolDefinition;    // JSON Schema (sent to LLM)
    async fn run(&self, call, ctx) -> Result<ToolResult, ToolError>;
}
```

### 5.3 Current Tool List

| Tool | Role | Permission Required |
|------|------|---------------------|
| `bash` | Shell command execution | Yes, except safe commands |
| `view` | Read files | No |
| `write` | Create/overwrite files | Yes |
| `edit` | Modify files via string replacement | Yes |
| `ls` | Directory listing | No |
| `glob` | Search files by pattern | No |
| `grep` | Search code by regex | No |

---

## 6. Message System

### 6.1 ContentPart (Polymorphic Messages)

```rust
enum ContentPart {
    Text { text }
    Reasoning { text }
    ToolCall { id, name, input }
    ToolResult { tool_call_id, content, is_error }
    Finish { reason, timestamp }
    Image { data, media_type }
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
sessions (id, title, message_count, tokens, cost, timestamps)
messages (id, session_id, role, parts_json, model_id, usage_json, timestamps)
```

WAL mode, embedded, no server required.

---

## 9. Full Sequence Diagram

```
User          CLI           Agent         Provider      Atlas Cloud     Tool
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
 │               │←─ContentDelta─│←─ContentDelta│←─SSE:stop────│            │
 │               │               │  [finish = EndTurn]         │            │
 │               │←─Complete─────│              │              │            │
 │←─[tokens:...]─│               │              │              │            │
```

---

## 10. Cost Calculation

```
Cost = (input tokens / 1M) × input price + (output tokens / 1M) × output price
```

DeepSeek V3.2 Speciale example:
```
Input  10,000 tokens × $0.27/M = $0.0027
Output  2,000 tokens × $0.41/M = $0.00082
Total                           = $0.00352
```

**Agent loop cost characteristics**: The entire conversation history is re-sent every loop iteration → input tokens accumulate.
The more tools are used, the cost increases exponentially.
