# octo-code-agent 아키텍처 분석서

## 1. 전체 구조

```
┌─────────────────────────────────────────────────────────────┐
│                      octo-cli (바이너리)                     │
│  main.rs → clap 파싱 → build_app() → repl / noninteractive │
└──────────┬──────────────────────────┬───────────────────────┘
           │                          │
    ┌──────▼──────┐          ┌────────▼────────┐
    │  REPL 모드   │          │  -p 모드 (1회)  │
    │  stdin 루프  │          │  자동승인       │
    └──────┬──────┘          └────────┬────────┘
           │                          │
           └──────────┬───────────────┘
                      │
              ┌───────▼────────┐
              │   octo-agent   │ ← 핵심 오케스트레이터
              │   Agent.run()  │
              └───────┬────────┘
                      │
         ┌────────────┼────────────┐
         │            │            │
  ┌──────▼──────┐ ┌───▼───┐ ┌─────▼─────┐
  │  Provider   │ │ Tools │ │ Permission│
  │(Atlas Cloud)│ │ (7개) │ │  Service  │
  └─────────────┘ └───────┘ └───────────┘
         │
  ┌──────▼──────┐
  │  octo-core  │ ← 공유 타입 정의
  └─────────────┘
         │
  ┌──────▼───────┐
  │ octo-storage │ ← SQLite 영속성
  └──────────────┘
```

### Cargo Workspace 구조 (6개 crate)

| crate | 역할 | 의존 대상 |
|-------|------|----------|
| `octo-core` | 타입, trait, 에러, 설정 | 없음 (최하위) |
| `octo-providers` | Atlas Cloud API 통신 | octo-core |
| `octo-tools` | 도구 실행 (bash, edit 등) | octo-core |
| `octo-agent` | 에이전트 루프 조율 | octo-core |
| `octo-storage` | SQLite DB | octo-core |
| `octo-cli` | 바이너리 진입점 | 전부 |

의존성 방향은 **단방향**: core ← providers/tools/agent/storage ← cli.
순환 의존이 없으므로 각 crate를 독립적으로 테스트 가능.

---

## 2. Atlas Cloud 통합 API

### 2.1 단일 키, 단일 엔드포인트

모든 LLM 호출은 **Atlas Cloud** 게이트웨이를 통해 라우팅됨:

```
엔드포인트: https://api.atlascloud.ai/api/v1/chat/completions
인증: Authorization: Bearer <ATLAS_API_KEY>
형식: OpenAI ChatCompletion 호환
```

**장점**: API 키 하나로 모든 모델 사용 가능. 별도 Provider 구현 불필요.

### 2.2 등록 모델 (5개)

| 모델 ID | 벤더 | 특징 | 입력 $/M | 출력 $/M |
|---------|------|------|---------|---------|
| `zai-org/glm-5` | Zhipu AI | 에이전트 최적화, 멀티스텝 추론 | $0.80 | $2.56 |
| `moonshotai/kimi-k2.5` | Moonshot AI | 초장문 컨텍스트, 멀티모달 | $0.50 | $2.50 |
| `qwen/qwen3-max-2026-01-23` | Alibaba | 플래그십, 코드 생성 | $1.20 | $6.00 |
| `minimaxai/minimax-m2.1` | MiniMax | 230B MoE, SWE-bench 74% | $0.30 | $0.30 |
| `deepseek-ai/deepseek-v3.2-speciale` | DeepSeek | 685B MoE, 최저가, IOI 금메달 | $0.27 | $0.41 |

**기본 모델**: `deepseek-ai/deepseek-v3.2-speciale` (가장 저렴하고 성능 우수)

### 2.3 설정

```
환경변수: ATLAS_API_KEY=your-key-here
또는 config 파일:
{
  "api_key": "your-key-here",
  "base_url": "https://api.atlascloud.ai/api"
}
```

키 감지 우선순위: `ATLAS_API_KEY` → `ATLAS_CLOUD_API_KEY` → `OPENAI_API_KEY` → `ANTHROPIC_API_KEY`

---

## 3. 핵심 원리: Agent Loop (에이전트 루프)

이 프로젝트의 **가장 중요한 메커니즘**. LLM이 "자율적으로" 코드를 수정할 수 있는 이유.

### 3.1 기본 개념

```
사용자: "이 파일의 버그를 고쳐줘"
  ↓
LLM: "파일을 먼저 읽어볼게요" + [tool_use: view {path: "main.rs"}]
  ↓
Agent: view 도구 실행 → 결과를 LLM에 다시 전달
  ↓
LLM: "37번 줄에 off-by-one 에러가 있네요" + [tool_use: edit {...}]
  ↓
Agent: edit 도구 실행 → 결과를 LLM에 다시 전달
  ↓
LLM: "수정 완료했습니다. 테스트를 돌려볼게요" + [tool_use: bash {command: "cargo test"}]
  ↓
Agent: bash 도구 실행 → 결과를 LLM에 다시 전달
  ↓
LLM: "모든 테스트가 통과합니다." [end_turn]
  ↓
Agent: 루프 종료
```

**핵심**: LLM이 `end_turn`을 반환할 때까지 무한 반복. LLM이 스스로 판단하여 도구를 호출하고, 결과를 보고, 다음 행동을 결정.

### 3.2 코드 흐름 (agent.rs)

```
Agent.run(session_id, messages, user_input)
  │
  ├─ tokio::spawn(agent_loop)    ← 별도 async task로 실행
  │    │
  │    ├─ messages.push(user_msg)
  │    │
  │    └─ loop {                  ← ★ 핵심 루프
  │         │
  │         ├─ provider.stream_response(messages, system_prompt, tools)
  │         │   → HTTP POST to Atlas Cloud (SSE 스트리밍)
  │         │
  │         ├─ process_stream() → (assistant_msg, finish_reason, usage)
  │         │   → 스트리밍 이벤트를 Message 객체로 조립
  │         │
  │         ├─ messages.push(assistant_msg)
  │         │
  │         └─ match finish_reason {
  │              EndTurn → return Ok(())     ← 루프 탈출
  │              ToolUse → {
  │                for each tool_call {
  │                  tool.run(call, ctx)     ← 도구 실행
  │                }
  │                messages.push(tool_results)
  │                continue                  ← 루프 계속
  │              }
  │            }
  │
  └─ return (rx, cancel_token)   ← CLI가 이벤트를 수신
```

### 3.3 FinishReason의 의미

| FinishReason | 의미 | Agent 동작 |
|-------------|------|-----------|
| `EndTurn` | 할 말 다 했음 | **루프 종료** |
| `ToolUse` | 도구를 쓰고 싶음 | 도구 실행 후 **루프 계속** |
| `MaxTokens` | 출력 한도 초과 | 루프 종료 |
| `Cancelled` | 사용자가 취소 | 루프 종료 |

**ToolUse일 때만 루프가 계속된다** - 이것이 에이전트가 "자율적으로 행동"하는 원리.

---

## 4. 스트리밍 아키텍처

### 4.1 왜 스트리밍인가

LLM 응답은 수 초~수십 초 소요. 전체 응답을 기다리면 UX가 나쁨.
→ **토큰 단위로 즉시 출력** (ChatGPT와 같은 타자기 효과)

### 4.2 3단계 이벤트 변환 파이프라인

```
[Atlas Cloud]           [Provider]              [Agent]              [CLI]
 SSE bytes  ──parse──→  ProviderEvent  ──process──→  AgentEvent  ──render──→  터미널
 (HTTP)                  (내부 추상화)               (UI용 이벤트)          (stdout)
```

**1단계: Provider (SSE → ProviderEvent)**
```rust
// openai.rs - SSE 바이트를 파싱하여 추상 이벤트로 변환
match delta {
    content → yield ProviderEvent::ContentDelta { text }
    tool_calls → yield ProviderEvent::ToolUseStart { id, name }
    finish_reason → yield ProviderEvent::Complete { finish_reason, usage }
}
```

**2단계: Agent (ProviderEvent → AgentEvent)**
```rust
// agent.rs - process_stream()
match event {
    ContentDelta { text } → {
        current_text += text;             // 메시지에 누적
        tx.send(AgentEvent::ContentDelta) // CLI에 즉시 전달
    }
    ToolUseStart → { /* 도구 호출 시작 */ }
    ToolUseStop  → { /* ToolCall을 Message.parts에 추가 */ }
    Complete     → { /* finish_reason 저장 */ }
}
```

**3단계: CLI (AgentEvent → 터미널 출력)**
```rust
// output.rs
match event {
    ContentDelta { text }     → print!("{text}")     // 실시간 타자기
    ToolCallStart { name }    → eprintln!("[tool: {name}]")
    ToolResult { result, .. } → eprintln!(result)
    Complete { usage, .. }    → eprintln!("[tokens: ...]")
}
```

### 4.3 채널 기반 비동기 통신

```
┌──────────┐   mpsc::channel(256)   ┌──────────┐
│  Agent   │ ─── AgentEvent ──────→ │   CLI    │
│  (task)  │                        │  (main)  │
└──────────┘                        └──────────┘
     ↑                                   │
     │  CancellationToken                │
     └──────── cancel signal ────────────┘
```

- `mpsc::channel`: Agent → CLI 단방향 이벤트 스트림
- `CancellationToken`: CLI → Agent 취소 신호 (Ctrl-C)
- Agent는 `tokio::spawn`으로 별도 task에서 실행 → CLI 블로킹 없음

---

## 5. Tool System (도구 시스템)

### 5.1 LLM의 도구 호출 원리

LLM은 **텍스트만 출력**할 수 있음. 파일을 읽거나, 명령을 실행할 수 없음.
→ "도구 정의"를 LLM에 알려주면, LLM이 **구조화된 JSON으로 도구 호출을 요청**함.

### 5.2 도구 인터페이스

```rust
trait Tool: Send + Sync {
    fn definition(&self) -> ToolDefinition;    // JSON Schema (LLM에 전달)
    async fn run(&self, call, ctx) -> Result<ToolResult, ToolError>;
}
```

### 5.3 현재 도구 목록

| 도구 | 역할 | 권한 필요 |
|------|------|----------|
| `bash` | 셸 명령 실행 | 안전한 명령 외 필요 |
| `view` | 파일 읽기 | 불필요 |
| `write` | 파일 생성/덮어쓰기 | 필요 |
| `edit` | 문자열 치환으로 파일 수정 | 필요 |
| `ls` | 디렉토리 목록 | 불필요 |
| `glob` | 패턴으로 파일 검색 | 불필요 |
| `grep` | 정규식으로 코드 검색 | 불필요 |

---

## 6. 메시지 시스템

### 6.1 ContentPart (다형성 메시지)

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

**tagged enum** + serde로 JSON 직렬화. 하나의 Message에 여러 종류의 콘텐츠가 섞일 수 있음.

---

## 7. 권한 시스템

LLM이 자율적으로 도구를 호출하되, **위험한 작업은 사용자 승인 필요**.

```
도구 실행 요청
    ├─ 안전한 명령? (ls, git status, echo 등) → 자동 승인
    ├─ -p 모드? → 자동 승인
    └─ 그 외 → CLI에서 사용자에게 물어봄
         Allow? [y]es / [n]o / [a]lways:
```

---

## 8. 저장소 (SQLite)

```sql
sessions (id, title, message_count, tokens, cost, timestamps)
messages (id, session_id, role, parts_json, model_id, usage_json, timestamps)
```

WAL 모드, 임베디드, 서버 불필요.

---

## 9. 전체 시퀀스 다이어그램

```
User          CLI           Agent         Provider      Atlas Cloud     Tool
 │               │               │              │              │            │
 │──"버그 고쳐"──→│               │              │              │            │
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

## 10. 비용 계산

```
비용 = (입력 토큰 / 1M) × 입력 단가 + (출력 토큰 / 1M) × 출력 단가
```

DeepSeek V3.2 Speciale 예시:
```
입력 10,000 토큰 × $0.27/M = $0.0027
출력  2,000 토큰 × $0.41/M = $0.00082
합계                        = $0.00352
```

**에이전트 루프의 비용 특성**: 매 루프마다 전체 대화 이력을 재전송 → 입력 토큰이 누적됨.
도구를 많이 사용할수록 비용이 기하급수적으로 증가.
