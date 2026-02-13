# octo-code-agent 아키텍처 분석서

## 1. 전체 구조

```
┌─────────────────────────────────────────────────────────────┐
│                    octo-code (바이너리)                      │
│  main.rs → clap 파싱 → build_app() → repl / interactive    │
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
              │     agent      │ ← 핵심 오케스트레이터
              │  Agent.run()   │
              └───────┬────────┘
                      │
         ┌────────────┼────────────┐
         │            │            │
  ┌──────▼──────┐ ┌───▼───┐ ┌─────▼─────┐
  │   Provider  │ │ Tools │ │ Permission│
  │(Atlas Cloud)│ │ (17개)│ │  Service  │
  │(OpenRouter) │ └───────┘ └───────────┘
  └─────────────┘
         │
  ┌──────▼──────┐
  │    core     │ ← 공유 타입 정의
  └──────┬──────┘
         │
  ┌──────▼───────┐
  │   storage    │ ← SQLite 영속성
  └──────────────┘
```

### 단일 Crate 구조

| 모듈 | 경로 | 역할 | 의존 대상 |
|------|------|------|----------|
| `core` | `src/core/` | 타입, trait, 에러, 설정 | 없음 (최하위) |
| `providers` | `src/providers/` | Atlas Cloud/OpenRouter API 통신 | core |
| `tools` | `src/tools/` | 도구 실행 (bash, edit 등 17개) | core |
| `agent` | `src/agent/` | 에이전트 루프 조율 | core |
| `storage` | `src/storage/` | SQLite DB | core |
| `cli` | `src/cli/` | 바이너리 진입점 | 전부 |

의존성 방향은 **단방향**: storage → core → providers/tools/agent → cli.
순환 의존이 없으므로 각 모듈을 독립적으로 테스트 가능.

---

## 2. LLM 제공자 통합

### 2.1 이중 제공자 지원

**Atlas Cloud** (기본) 및 **OpenRouter**를 동시에 지원합니다:

```
Atlas Cloud:  https://api.atlascloud.ai/api/v1/chat/completions
OpenRouter:   https://openrouter.ai/api/v1/chat/completions

인증: Authorization: Bearer <API_KEY>
형식: OpenAI ChatCompletion 호환
```

**장점**: 
- API 키 하나로 Atlas Cloud의 모든 모델 사용 가능
- OpenRouter 키로도 동일한 모델 사용 가능
- `--provider` 플래그로 런타임 전환

### 2.2 등록 모델 (6개)

| 모델 ID | 벤더 | 특징 | 입력 $/M | 출력 $/M | 컨텍스트 |
|---------|------|------|---------|---------|---------|
| `zai-org/glm-5` | Zhipu AI | 에이전트 최적화, 멀티스텝 추론 | $0.80 | $2.56 | 202K |
| `zai-org/glm-4.7` | Zhipu AI | 경제적, 빠른 응답 | $0.52 | $1.75 | 202K |
| `deepseek-ai/deepseek-v3.2-speciale` | DeepSeek | 685B MoE, 최저가 | $0.26 | $0.38 | 163K |
| `qwen/qwen3-max-2026-01-23` | Alibaba | 플래그십, 강력한 추론 | $1.20 | $6.00 | 252K |
| `Qwen/Qwen3-Coder` | Alibaba | 480B MoE, 코드 특화 | $0.78 | $3.90 | 262K |
| `moonshotai/kimi-k2.5` | Moonshot AI | 초장문 컨텍스트, 멀티모달 | $0.50 | $2.50 | 262K |

**기본 모델**: `zai-org/glm-5` (에이전트 최적화)

**경제적 모델**: `deepseek-ai/deepseek-v3.2-speciale` (최저가)

### 2.3 설정

```bash
# 환경변수
export ATLAS_API_KEY="your-key-here"
export OPENROUTER_API_KEY="your-key-here"

# 또는 첫 실행 시 자동 설정
octo-code

# 설정 파일 (JSON 형식)
# macOS: ~/Library/Application Support/octo-code/config.json
# Linux: ~/.config/octo-code/config.json
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
  │         │   → HTTP POST (SSE 스트리밍)
  │         │
  │         ├─ process_stream() → (assistant_msg, finish_reason, usage)
  │         │   → 스트리밍 이벤트를 Message 객체로 조립
  │         │
  │         ├─ messages.push(assistant_msg)
  │         │
  │         ├─ 컨텍스트 윈도우 관리 (오래된 메시지 제거)
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

### 3.4 Rate Limit 대응

```rust
// 3회 재시도 + 지수 백오프
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

## 4. 스트리밍 아키텍처

### 4.1 왜 스트리밍인가

LLM 응답은 수 초~수십 초 소요. 전체 응답을 기다리면 UX가 나쁨.
→ **토큰 단위로 즉시 출력** (ChatGPT와 같은 타자기 효과)

### 4.2 3단계 이벤트 변환 파이프라인

```
[LLM API]               [Provider]              [Agent]              [CLI]
 SSE bytes  ──parse──→  ProviderEvent  ──process──→  AgentEvent  ──render──→  터미널
 (HTTP)                  (내부 추상화)               (UI용 이벤트)           (stdout)
```

**1단계: Provider (SSE → ProviderEvent)**
```rust
// providers/openai.rs - SSE 바이트를 파싱하여 추상 이벤트로 변환
match delta {
    content → yield ProviderEvent::ContentDelta { text }
    tool_calls → yield ProviderEvent::ToolUseStart { id, name }
    finish_reason → yield ProviderEvent::Complete { finish_reason, usage }
}
```

**2단계: Agent (ProviderEvent → AgentEvent)**
```rust
// agent/agent.rs - process_stream()
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
// cli/output.rs
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
#[async_trait]
pub trait Tool: Send + Sync {
    fn definition(&self) -> ToolDefinition;    // JSON Schema (LLM에 전달)
    async fn run(&self, call: &ToolCall, ctx: &ToolContext) 
        -> Result<ToolResult, ToolError>;
}
```

### 5.3 현재 도구 목록 (17개)

| 카테고리 | 도구 | 역할 | 권한 필요 |
|----------|------|------|----------|
| 파일 | `view` | 파일 읽기 | ❌ 없음 |
| | `write` | 파일 생성/덮어쓰기 | ✅ 필요 |
| | `edit` | 문자열 치환으로 파일 수정 | ✅ 필요 |
| 탐색 | `ls` | 디렉토리 목록 | ❌ 없음 |
| | `glob` | 패턴으로 파일 검색 | ❌ 없음 |
| | `grep` | 정규식으로 코드 검색 | ❌ 없음 |
| 실행 | `bash` | 셸 명령 실행 | ✅ 위험 명령 |
| 코드 | `coderlm` | CodeRLM 코드 인텔리전스 | ❌ 없음 |
| 팀 | `team_create` | 팀 생성 | ✅ 필요 |
| | `team_delete` | 팀 삭제 | ✅ 필요 |
| | `spawn_agent` | 에이전트 생성 | ✅ 필요 |
| 태스크 | `task_create` | 작업 생성 | ✅ 필요 |
| | `task_get` | 작업 조회 | ❌ 없음 |
| | `task_update` | 작업 업데이트 | ✅ 필요 |
| | `task_list` | 작업 목록 | ❌ 없음 |
| 메시지 | `send_message` | 메시지 전송 | ✅ 필요 |
| | `check_inbox` | 메시지 수신 | ❌ 없음 |

---

## 6. 메시지 시스템

### 6.1 ContentPart (다형성 메시지)

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
-- 세션 테이블
sessions (
    id, title, message_count, 
    prompt_tokens, completion_tokens, cost,
    created_at, updated_at
)

-- 메시지 테이블
messages (
    id, session_id, role, parts_json, 
    model_id, usage_json, created_at, updated_at
)

-- 파일 버전 관리
files (
    id, session_id, path, content, 
    version, created_at, updated_at
)
```

WAL 모드, 임베디드, 서버 불필요.

---

## 9. 팀 협업 시스템 (병렬 멀티 에이전트)

### 9.1 개념

복잡한 작업을 자동 분해하여 여러 에이전트가 병렬로 처리:

```
사용자: "Next.js 랜딩페이지 만들어줘"
    ↓
리드 에이전트: 작업 분해
    ├─ spawn_agent: layout (레이아웃 + 네비게이션)
    ├─ spawn_agent: hero (히어로 섹션 + CTA)
    └─ spawn_agent: features (피처 카드 + 푸터)
    ↓
에이전트들이 병렬로 작업 → 파일 기반 태스크 보드로 조율
    ↓
리드 에이전트: 결과 통합 및 검증
```

### 9.2 파일 기반 조율

```
~/.octo-code/
├── teams/{team-name}/
│   ├── config.json         # 팀 설정, 멤버 목록
│   └── inboxes/
│       └── {agent}.json    # 에이전트별 메시지 큐
└── tasks/{team-name}/
    ├── counter.json        # 태스크 ID 카운터
    └── {id}.json           # 개별 태스크
```

---

## 10. 전체 시퀀스 다이어그램

```
User          CLI           Agent         Provider      LLM API       Tool
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
 │               │               │              │←─SSE:stop────│            │
 │               │               │  [finish = EndTurn]         │            │
 │               │←─Complete─────│              │              │            │
 │←─[tokens:...]─│               │              │              │            │
```

---

## 11. 비용 계산

```
비용 = (입력 토큰 / 1M) × 입력 단가 + (출력 토큰 / 1M) × 출력 단가
```

DeepSeek V3.2 Speciale 예시:
```
입력 10,000 토큰 × $0.26/M = $0.0026
출력  2,000 토큰 × $0.38/M = $0.00076
합계                         = $0.00336
```

**에이전트 루프의 비용 특성**: 매 루프마다 전체 대화 이력을 재전송 → 입력 토큰이 누적됨.
도구를 많이 사용할수록 비용이 증가 (단, 컨텍스트 트리밍으로 관리).
