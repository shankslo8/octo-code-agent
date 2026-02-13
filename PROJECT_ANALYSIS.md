# octo-code-agent 프로젝트 분석서

## 📋 개요

**octo-code-agent**는 터미널에서 작동하는 AI 코딩 어시스턴트 CLI 도구입니다. Rust로 작성되었으며, LLM(대형 언어 모델)을 활용해 코드 작성, 수정, 디버깅을 자율적으로 수행합니다.

| 항목 | 내용 |
|------|------|
| **언어** | Rust (Edition 2021) |
| **아키텍처** | 단일 Crate (bin + lib) |
| **비동기 런타임** | Tokio |
| **데이터베이스** | SQLite (sqlx) |
| **LLM 제공자** | Atlas Cloud, OpenRouter (OpenAI-compatible) |

---

## 🏗️ 프로젝트 구조

### 단일 Crate 구조

```
octo-code-agent/
├── Cargo.toml              # 단일 crate (bin + lib)
├── src/
│   ├── main.rs             # 바이너리 진입점 (6줄)
│   ├── lib.rs              # 라이브러리 루트
│   ├── core/               # 핵심 타입 및 트레이트
│   │   ├── config.rs       # AppConfig 설정
│   │   ├── model.rs        # 모델 정의 및 가격 정보
│   │   ├── message.rs      # 메시지 시스템
│   │   ├── tool.rs         # Tool 트레이트
│   │   ├── provider.rs     # Provider 트레이트
│   │   ├── permission.rs   # 권한 관리
│   │   └── ...
│   ├── providers/          # LLM API 제공자
│   │   └── openai.rs       # OpenAI-compatible API
│   ├── tools/              # 도구 구현 (17개)
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
│   ├── agent/              # 에이전트 루프
│   │   ├── agent.rs        # 핵심 Agent 루프
│   │   ├── event.rs        # AgentEvent 정의
│   │   └── prompt.rs       # 시스템 프롬프트
│   ├── storage/            # SQLite 저장소
│   │   ├── database.rs
│   │   ├── session_repo.rs
│   │   └── message_repo.rs
│   └── cli/                # CLI 인터페이스
│       ├── interactive.rs
│       ├── repl.rs
│       ├── tui/            # ratatui 기반 TUI
│       └── ...
├── migrations/             # SQLite 마이그레이션
└── docs/                   # 문서
```

### 모듈 의존성 그래프

```
                    ┌─────────────┐
                    │    cli      │  ← 진입점 (main.rs)
                    └──────┬──────┘
                           │
       ┌───────────────────┼───────────────────┐
       │                   │                   │
┌──────▼──────┐    ┌───────▼───────┐   ┌──────▼──────┐
│    agent    │    │     tools     │   │  providers  │
└──────┬──────┘    └───────┬───────┘   └──────┬──────┘
       │                   │                   │
       └───────────────────┼───────────────────┘
                           │
                    ┌──────▼──────┐
                    │    core     │  ← 공유 타입 정의
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │   storage   │  ← SQLite 영속성
                    └─────────────┘
```

**의존성 규칙**: 단방향 의존 (storage ← core ← others ← cli). 순환 의존 없음.

---

## 📦 모듈별 상세 분석

### 1. core (핵심 타입)

**파일**: `src/core/`

| 모듈 | 역할 |
|------|------|
| `config.rs` | AppConfig 설정 로드/관리 (JSON 기반) |
| `error.rs` | 에러 타입 정의 |
| `message.rs` | Message, ContentPart, Role 등 메시지 시스템 |
| `model.rs` | ModelId, 7개 모델 정의 및 가격 정보 |
| `permission.rs` | PermissionService 트레이트 |
| `provider.rs` | Provider 트레이트 (LLM 통신 추상화) |
| `session.rs` | 세션 관리 타입 |
| `team.rs` | 팀 협업 상태 관리 |
| `tool.rs` | Tool 트레이트 및 도구 관련 타입 |

**핵심 트레이트**:
```rust
// Provider: LLM 통신
pub trait Provider: Send + Sync {
    async fn stream_response(...) -> Result<ProviderStream, ProviderError>;
}

// Tool: 도구 실행
pub trait Tool: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    async fn run(&self, call: ToolCall, ctx: ToolContext) -> Result<ToolResult, ToolError>;
}

// PermissionService: 권한 관리
pub trait PermissionService: Send + Sync {
    async fn request_permission(&self, request: PermissionRequest) -> PermissionDecision;
}
```

### 2. providers (LLM API)

**파일**: `src/providers/`

Atlas Cloud 및 OpenRouter API와 통신합니다.

```rust
// 공개 API
pub fn create_provider(config: &AppConfig, model_id: Option<&ModelId>) 
    -> Result<Arc<dyn Provider>, ProviderError>;
```

**지원 모델 역할**:
- `Coder` - 기본 코딩 모델 (GLM-5)
- `Fast` - 가벼운 작업용 (GLM-4.7)
- `Reasoning` - 복잡한 추론용 (Qwen3 Max)
- `LongContext` - 장문 컨텍스트용 (Kimi K2.5)

### 3. tools (도구 모음)

**파일**: `src/tools/`

| 도구 | 파일 | 설명 | 권한 필요 |
|------|------|------|-----------|
| `bash` | `bash.rs` | 셸 명령 실행 | ✅ 위험 명령 |
| `view` | `view.rs` | 파일 읽기 | ❌ 없음 |
| `write` | `write.rs` | 파일 생성/쓰기 | ✅ 필요 |
| `edit` | `edit.rs` | 문자열 치환 수정 | ✅ 필요 |
| `ls` | `ls.rs` | 디렉토리 목록 | ❌ 없음 |
| `glob` | `glob_tool.rs` | 패턴 파일 검색 | ❌ 없음 |
| `grep` | `grep.rs` | 정규식 코드 검색 | ❌ 없음 |
| `coderlm` | `coderlm.rs` | CodeRLM 코드 인텔리전스 | ❌ 없음 |
| `team_create` | `team.rs` | 팀 생성 | ✅ 필요 |
| `team_delete` | `team.rs` | 팀 삭제 | ✅ 필요 |
| `spawn_agent` | `team.rs` | 에이전트 생성 | ✅ 필요 |
| `task_create` | `task_mgmt.rs` | 작업 생성 | ✅ 필요 |
| `task_get` | `task_mgmt.rs` | 작업 조회 | ❌ 없음 |
| `task_update` | `task_mgmt.rs` | 작업 업데이트 | ✅ 필요 |
| `task_list` | `task_mgmt.rs` | 작업 목록 | ❌ 없음 |
| `send_message` | `send_message.rs` | 메시지 전송 | ✅ 필요 |
| `check_inbox` | `send_message.rs` | 메시지 수신 | ❌ 없음 |

### 4. agent (에이전트 엔진)

**파일**: `src/agent/`

| 파일 | 역할 |
|------|------|
| `agent.rs` | 핵심 Agent 루프 구현 (469줄) |
| `event.rs` | AgentEvent 정의 (UI 이벤트) |
| `prompt.rs` | 시스템 프롬프트 생성 |

**Agent 루프 핵심**:
```rust
// Agent.run() 내부 루프
loop {
    // 1. LLM에 메시지 스트리밍 요청
    let stream = provider.stream_response(messages, ...).await?;
    
    // 2. 스트림 처리하여 이벤트 발생
    let (msg, finish_reason) = process_stream(stream, tx).await?;
    
    // 3. 메시지 저장
    messages.push(msg);
    
    // 4. 종료 조건 확인
    match finish_reason {
        FinishReason::EndTurn => break,      // 루프 종료
        FinishReason::ToolUse => {           // 도구 실행 후 계속
            for tool_call in tool_calls {
                let result = tool.run(tool_call, ctx).await?;
                messages.push(tool_result_msg);
            }
            continue;
        }
        _ => break,
    }
}
```

### 5. storage (데이터 영속성)

**파일**: `src/storage/`

| 파일 | 역할 |
|------|------|
| `database.rs` | SQLite 연결 및 마이그레이션 |
| `session_repo.rs` | 세션 CRUD |
| `message_repo.rs` | 메시지 CRUD |

**스키마**:
```sql
-- 세션 테이블
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    message_count INTEGER DEFAULT 0,
    prompt_tokens INTEGER DEFAULT 0,
    completion_tokens INTEGER DEFAULT 0,
    cost REAL DEFAULT 0,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

-- 메시지 테이블
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,  -- 'user' | 'assistant'
    parts_json TEXT NOT NULL,  -- ContentPart JSON 배열
    model_id TEXT,
    usage_json TEXT,
    created_at TIMESTAMP,
    updated_at TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- 파일 버전 관리
CREATE TABLE files (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    path TEXT NOT NULL,
    content TEXT NOT NULL,
    version INTEGER DEFAULT 1,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);
```

### 6. cli (CLI 인터페이스)

**파일**: `src/cli/`

| 파일 | 역할 |
|------|------|
| `mod.rs` | CLI 진입점, 모드 분기 |
| `interactive.rs` | 대화형 모드 (모델 선택 UI) |
| `noninteractive.rs` | -p 플래그 모드 |
| `repl.rs` | REPL 모드 |
| `setup.rs` | 초기 설정 (API 키 입력) |
| `output.rs` | 출력 포맷팅 |
| `permission_ui.rs` | CLI 권한 UI |
| `tui/` | TUI 모드 (ratatui) |

**CLI 모드**:
```bash
# 대화형 모드 (기본) - 모델 선택 → 작업 입력
octo-code

# 한 번 실행 모드
octo-code -p "버그를 고쳐줘"

# REPL 모드
octo-code --repl

# TUI 모드
octo-code --tui

# 이전 세션 재개
octo-code --session <session_id>

# 모델 지정
octo-code -m "zai-org/glm-5"

# OpenRouter 사용
octo-code --provider openrouter
```

---

## 🔄 핵심 흐름: 에이전트 루프

에이전트가 "자율적으로" 코드를 수정하는 원리입니다.

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

**핵심 원리**: LLM이 `end_turn`을 반환할 때까지 무한 반복. 매 반복마다 전체 대화 이력을 재전송합니다.

---

## 📡 스트리밍 아키텍처

토큰 단위 실시간 출력을 위한 3단계 파이프라인:

```
[Atlas Cloud]    →    [Provider]    →    [Agent]    →    [CLI]
   SSE bytes          ProviderEvent      AgentEvent      터미널 출력
   (HTTP)             (추상화)           (UI용)          (stdout)
```

**채널 기반 통신**:
```rust
// Agent → CLI 이벤트 스트림
let (tx, rx) = mpsc::channel::<AgentEvent>(256);

// CLI → Agent 취소 신호
let cancel_token = CancellationToken::new();
```

---

## 🔐 권한 시스템

LLM의 자율성과 안전성의 균형:

```
도구 실행 요청
    ├─ 안전한 명령? (ls, git status, echo 등) → 자동 승인
    ├─ -p 모드? → 자동 승인
    └─ 그 외 → 사용자에게 물어봄
         Allow? [y]es / [n]o / [a]lways:
```

**권한이 필요한 도구**: bash (위험 명령), write, edit, team_*, task_*, spawn_agent, send_message

---

## 💰 비용 모델

### 지원 모델 및 가격 (2025년 2월 기준)

| 모델 | 벤더 | 입력 $/M | 출력 $/M | 컨텍스트 | 특징 |
|------|------|---------|---------|---------|------|
| **GLM-5** | Zhipu AI | $0.80 | $2.56 | 202K | 745B MoE, 기본 모델 |
| **GLM-4.7** | Zhipu AI | $0.52 | $1.75 | 202K | 경제적, 131K output |
| **DeepSeek V3.2** | DeepSeek | $0.26 | $0.38 | 163K | 685B MoE, 최저가 |
| **Qwen3 Max** | Alibaba | $1.20 | $6.00 | 252K | Flagship reasoning |
| **Qwen3 Coder** | Alibaba | $0.78 | $3.80 | 262K | 480B MoE, 코드 특화 |
| **Kimi K2.5** | Moonshot | $0.50 | $2.50 | 262K | Deep reasoning |

**비용 계산**:
```
비용 = (입력 토큰 / 1M) × 입력 단가 + (출력 토큰 / 1M) × 출력 단가
```

**주의**: 에이전트 루프는 매 반복마다 전체 이력을 재전송 → 입력 토큰 누적

---

## 🔧 주요 외부 의존성

| 크레이트 | 용도 |
|----------|------|
| `tokio` | 비동기 런타임 |
| `serde` / `serde_json` | 직렬화 |
| `anyhow` / `thiserror` | 에러 처리 |
| `reqwest` | HTTP 클라이언트 |
| `sqlx` | SQLite ORM + 마이그레이션 |
| `clap` | CLI 파싱 |
| `ratatui` | TUI 프레임워크 |
| `crossterm` | 터미널 제어 |
| `tokio-stream` | 스트리밍 |
| `uuid` | UUID 생성 |
| `chrono` | 날짜/시간 |
| `glob` | 파일 패턴 검색 |
| `regex` | 정규식 |

---

## 🚀 빌드 및 실행

```bash
# 빌드
cargo build --release

# 개발 빌드
cargo build

# 테스트
cargo test

# 실행 (개발)
cargo run

# 설치
cargo install --path .
```

---

## 📊 요약

octo-code-agent는 **Rust 기반의 AI 코딩 어시스턴트**로:

1. **단일 Crate 구조**: 단순한 bin + lib 구조, workspace 아님
2. **Agent Loop**: LLM이 자율적으로 도구를 호출하며 작업 수행
3. **Streaming**: 실시간 토큰 출력으로 UX 개선
4. **Multi-modal**: 대화형, REPL, TUI, 비대화형 모드 지원
5. **Safety**: 권한 시스템으로 위험 작업 보호
6. **Persistence**: SQLite로 세션/메시지 저장
7. **Cost-aware**: Atlas Cloud/OpenRouter 통합으로 투명한 과금
8. **Multi-provider**: Atlas Cloud와 OpenRouter 동시 지원

**핵심 가치**: 개발자가 자연어로 코딩 작업을 의뢰하면, AI가 파일 읽기 → 분석 → 수정 → 테스트까지 **자율적으로 수행**합니다.
