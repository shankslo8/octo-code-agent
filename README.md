# 🐙 OctoCode Agent

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Crates.io](https://img.shields.io/crates/v/octo-code-agent.svg)](https://crates.io/crates/octo-code-agent)
[![Docs.rs](https://docs.rs/octo-code-agent/badge.svg)](https://docs.rs/octo-code-agent)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Tests](https://img.shields.io/badge/tests-37%20passing-brightgreen.svg)]()

**AI 코딩 에이전트** — 여러 에이전트가 동시에 코드를 분석하고 수정하는 터미널 기반 코딩 어시스턴트

```
  🐙 OctoCode Agent v0.1.0 (GLM 5)
  Type your task, /help for commands, Ctrl-D to exit

  octo> Next.js 랜딩페이지 만들어줘

  [team_create: landing-page]
  [spawn_agent: layout]    ← 레이아웃 + 네비게이션
  [spawn_agent: hero]      ← 히어로 섹션 + CTA
  [spawn_agent: features]  ← 피처 카드 + 푸터
  ...
  ✓ Build succeeded. 3 agents, 12 files created.
```

---

## 📋 목차

- [특징](#-특징)
- [설치](#-설치)
- [빠른 시작](#-빠른-시작)
- [사용법](#-사용법)
- [지원 모델](#-지원-모델)
- [아키텍처](#-아키텍처)
- [도구 목록](#-도구-목록)
- [팀 협업](#-팀-협업-고급-기능)
- [API 제공자](#-api-제공자)
- [설정](#-설정)
- [테스트](#-테스트)
- [배포 및 패키징](#-배포-및-패키징)
- [문서](#-문서)
- [라이선스](#-라이선스)

---

## ✨ 특징

### 🚀 핵심 기능

| 기능 | 설명 |
|------|------|
| **병렬 멀티 에이전트** | 작업을 자동 분해하여 여러 에이전트가 동시에 작업 수행 |
| **팀 조율 시스템** | 파일 기반 태스크 보드, 인박스 메시징, 자동 에이전트 스폰 |
| **CodeRLM 통합** | tree-sitter 기반 코드 인텔리전스 (선택사항) |
| **7개 모델 지원** | GLM 5, GLM 4.7, DeepSeek V3.2, Qwen3 Max, Qwen3 Coder, Kimi K2.5 |
| **듀얼 API 제공자** | Atlas Cloud 및 OpenRouter 동시 지원 |
| **비용 추적** | 실시간 토큰 사용량 + 비용 계산 및 표시 |
| **세션 관리** | SQLite 기반 대화 히스토리, 세션 이어하기 |
| **Rate Limit 대응** | 자동 재시도, 지수 백오프, 다중 API 키 로드밸런싱 |

### 🛡️ 안전성

- **권한 시스템**: 위험한 작업은 사용자 승인 필요
- **Path Traversal 방지**: 작업 디렉토리 외부 접근 차단
- **명령어 필터링**: 위험한 bash 명령 자동 차단

---

## 📦 설치

### 사전 요구사항

- [Rust](https://rustup.rs/) 1.75 이상
- Atlas Cloud 또는 OpenRouter API 키

---

### 방법 1: crates.io에서 설치 (권장) ⭐

Rust 생태계의 공식 패키지 저장소 [crates.io](https://crates.io/crates/octo-code-agent)에서 직접 설치합니다.

```bash
# 설치 (전 세계 어디서든)
cargo install octo-code-agent

# 또는 특정 버전 설치
cargo install octo-code-agent --version 0.1.0

# 업데이트
cargo install octo-code-agent --force
```

**설치 위치:**
- 바이너리: `~/.cargo/bin/octo-code`
- PATH에 `~/.cargo/bin`이 포함되어 있어야 함 (cargo 설치 시 자동 추가)

**확인:**
```bash
octo-code --version
```

---

### 방법 2: 소스에서 설치

최신 개발 버전이나 커스텀 수정이 필요한 경우:

```bash
# 저장소 클론
git clone https://github.com/johunsang/octo-code-agent
cd octo-code-agent

# 설치
cargo install --path .

# 또는 릴리스 빌드만 (설치 없이)
cargo build --release
# 바이너리: target/release/octo-code
```

---

### 방법 3: 바이너리 직접 다운로드

GitHub [Releases](https://github.com/johunsang/octo-code-agent/releases)에서 미리 빌드된 바이너리를 다운로드:

```bash
# macOS (Apple Silicon)
curl -L -o octo-code https://github.com/johunsang/octo-code-agent/releases/latest/download/octo-code-macos-arm64
chmod +x octo-code
sudo mv octo-code /usr/local/bin/

# macOS (Intel)
curl -L -o octo-code https://github.com/johunsang/octo-code-agent/releases/latest/download/octo-code-macos-x86_64
chmod +x octo-code
sudo mv octo-code /usr/local/bin/

# Linux
curl -L -o octo-code https://github.com/johunsang/octo-code-agent/releases/latest/download/octo-code-linux-x86_64
chmod +x octo-code
sudo mv octo-code /usr/local/bin/
```

---

### 방법 4: Docker

```bash
docker pull johunsang/octo-code:latest
docker run -it --rm -e ATLAS_API_KEY=$ATLAS_API_KEY johunsang/octo-code:latest
```

---

### 방법 5: macOS Homebrew (예정)

```bash
brew tap johunsang/octo-code
brew install octo-code
```

---

## 🚀 빠른 시작

### 1. API 키 설정

```bash
# Atlas Cloud 사용
export ATLAS_API_KEY="sk-your-api-key"

# 또는 OpenRouter 사용
export OPENROUTER_API_KEY="sk-your-api-key"

# 여러 키 로드밸런싱
export ATLAS_API_KEYS="key1,key2,key3"
```

### 2. 첫 실행

```bash
$ octo-code

🐙 OctoCode Agent v0.1.0

Select a model:
1. GLM-5 (zai-org/glm-5) - $0.80/$2.56 per 1M tokens [default]
2. GLM-4.7 (zai-org/glm-4.7) - $0.52/$1.75 per 1M tokens
3. DeepSeek V3.2 (deepseek-ai/deepseek-v3.2-speciale) - $0.26/$0.38 per 1M tokens
4. Qwen3 Max (qwen/qwen3-max-2026-01-23) - $1.20/$6.00 per 1M tokens
5. Qwen3 Coder (Qwen/Qwen3-Coder) - $0.78/$3.90 per 1M tokens
6. Kimi K2.5 (moonshotai/kimi-k2.5) - $0.50/$2.50 per 1M tokens

octo> Rust로 간단한 HTTP 서버를 만들어줘
```

---

## 💻 사용법

### 실행 모드

| 모드 | 명령어 | 설명 |
|------|--------|------|
| **인터랙티브** | `octo-code` | 모델 선택 → 대화형 입력 (기본) |
| **단일 명령** | `octo-code -p "버그 수정"` | 한 번 실행 후 종료 |
| **REPL** | `octo-code --repl` | Read-Eval-Print Loop |
| **TUI** | `octo-code --tui` | ratatui 기반 풀스크린 UI |
| **세션 재개** | `octo-code --session <id>` | 이전 세션 이어하기 |

### CLI 옵션

```
USAGE:
    octo-code [OPTIONS]

OPTIONS:
    -p, --prompt <PROMPT>         한 번 실행할 프롬프트
    -c, --cwd <PATH>              작업 디렉토리 지정
    -f, --output-format <FMT>     출력 형식 (text, json) [default: text]
    -q, --quiet                   진행 표시기 숨김
        --repl                    REPL 모드로 실행
        --tui                     TUI 모드로 실행
        --session <SESSION_ID>    이전 세션 재개
    -m, --model <MODEL_ID>        사용할 모델 지정
        --provider <PROVIDER>     API 제공자 (atlas, openrouter)
    -d, --debug                   디버그 로그 활성화
    -h, --help                    도움말 표시
    -V, --version                 버전 표시
```

### 대화 명령어

| 명령어 | 설명 |
|--------|------|
| `/help`, `/h` | 도움말 표시 |
| `/model` | 현재 모델 확인 |
| `/cost` | 토큰 사용량 및 비용 확인 |
| `/sessions`, `/s` | 세션 목록 |
| `/session <id>` | 세션 전환 |
| `/clear` | 현재 세션 메시지 초기화 |
| `/exit`, `/q` | 종료 |

---

## 🤖 지원 모델

| 모델 | 벤더 | 입력 $/M | 출력 $/M | 컨텍스트 | 특징 |
|------|------|---------|---------|---------|------|
| **GLM-5** | Zhipu AI | $0.80 | $2.56 | 202K | 745B MoE, 에이전트 최적화, 기본 모델 |
| **GLM-4.7** | Zhipu AI | $0.52 | $1.75 | 202K | 경제적, 빠른 응답 |
| **DeepSeek V3.2** | DeepSeek | $0.26 | $0.38 | 163K | 685B MoE, **최저가** |
| **Qwen3 Max** | Alibaba | $1.20 | $6.00 | 252K | Flagship, 강력한 추론 |
| **Qwen3 Coder** | Alibaba | $0.78 | $3.90 | 262K | 480B MoE, 코드 특화 |
| **Kimi K2.5** | Moonshot | $0.50 | $2.50 | 262K | 초장문 컨텍스트, 멀티모달 |

**비용 계산 예시 (DeepSeek V3.2):**
```
입력 10,000 토큰 × $0.26/M = $0.0026
출력  2,000 토큰 × $0.38/M = $0.00076
합계                         = $0.00336
```

---

## 🏗️ 아키텍처

### 단일 Crate 구조

```
octo-code-agent/
├── Cargo.toml              # 단일 crate (bin + lib)
├── src/
│   ├── main.rs             # 바이너리 진입점
│   ├── lib.rs              # 라이브러리 루트
│   ├── core/               # 핵심 타입 및 트레이트
│   │   ├── config.rs       # 설정 관리
│   │   ├── model.rs        # 모델 정의 및 가격
│   │   ├── message.rs      # 메시지 시스템
│   │   ├── tool.rs         # Tool 트레이트
│   │   ├── provider.rs     # Provider 트레이트
│   │   ├── permission.rs   # 권한 관리
│   │   └── tests.rs        # 단위 테스트
│   ├── providers/          # LLM API 제공자
│   │   └── openai.rs       # OpenAI-compatible (Atlas, OpenRouter)
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
│   │   ├── send_message.rs
│   │   └── tests.rs        # 도구 테스트
│   ├── agent/              # 에이전트 루프
│   │   ├── agent.rs        # 핵심 Agent 루프
│   │   ├── event.rs        # AgentEvent 정의
│   │   └── prompt.rs       # 시스템 프롬프트
│   ├── storage/            # SQLite 저장소
│   │   ├── database.rs
│   │   ├── session_repo.rs
│   │   ├── message_repo.rs
│   │   └── tests.rs        # 저장소 테스트
│   └── cli/                # CLI 인터페이스
│       ├── interactive.rs
│       ├── repl.rs
│       ├── tui/            # ratatui 기반 TUI
│       └── ...
├── migrations/             # SQLite 마이그레이션
└── docs/                   # 문서
```

### 핵심 흐름: Agent Loop

```
사용자 입력
    ↓
┌─────────────────────────────────────┐
│  LLM 스트리밍 요청 (with tool defs)  │
└─────────────┬───────────────────────┘
              ↓
┌─────────────────────────────────────┐
│  process_stream()                   │
│  - ContentDelta (텍스트 출력)        │
│  - ThinkingDelta (사고 과정)         │
│  - ToolUseStart/Stop (도구 호출)     │
└─────────────┬───────────────────────┘
              ↓
    ┌─────────────────┐
    │  finish_reason? │
    └────────┬────────┘
             │
    ┌────────┴────────┐
    ▼                 ▼
┌──────────┐    ┌──────────┐
│ EndTurn  │    │ ToolUse  │
│ (종료)    │    │ (도구실행)│
└──────────┘    └────┬─────┘
                     ↓
            ┌────────────────┐
            │ tool.run(ctx)  │
            │ 결과를 메시지에 추가│
            └───────┬────────┘
                    ↓
              (루프 반복)
```

---

## 🛠️ 도구 목록

### 파일 도구

| 도구 | 설명 | 권한 |
|------|------|------|
| `view` | 파일 읽기 (offset/limit 지원) | ❌ 없음 |
| `write` | 파일 생성/덮어쓰기 | ✅ 필요 |
| `edit` | 문자열 치환으로 파일 수정 | ✅ 필요 |

### 탐색 도구

| 도구 | 설명 | 권한 |
|------|------|------|
| `ls` | 디렉토리 목록 | ❌ 없음 |
| `glob` | 패턴으로 파일 검색 | ❌ 없음 |
| `grep` | 정규식으로 코드 검색 | ❌ 없음 |

### 실행 도구

| 도구 | 설명 | 권한 |
|------|------|------|
| `bash` | 셸 명령 실행 (타임아웃 지원) | ✅ 위험 명령 |

### 코드 인텔리전스

| 도구 | 설명 | 권한 |
|------|------|------|
| `coderlm` | CodeRLM 통합 (structure, search, symbols 등) | ❌ 없음 |

### 팀 협업 도구

| 도구 | 설명 | 권한 |
|------|------|------|
| `team_create` | 팀 생성 | ✅ 필요 |
| `team_delete` | 팀 삭제 | ✅ 필요 |
| `spawn_agent` | 에이전트 생성 | ✅ 필요 |
| `task_create` | 태스크 생성 | ✅ 필요 |
| `task_get` | 태스크 조회 | ❌ 없음 |
| `task_update` | 태스크 업데이트 | ✅ 필요 |
| `task_list` | 태스크 목록 | ❌ 없음 |
| `send_message` | 메시지 전송 | ✅ 필요 |
| `check_inbox` | 메시지 수신 | ❌ 없음 |

---

## 👥 팀 협업 (고급 기능)

여러 AI 에이전트를 병렬로 실행하여 복잡한 작업을 분할 처리합니다.

### 사용 예시

```bash
octo> Next.js 랜딩페이지를 만들어줘
```

AI가 자동으로 수행:

```
[team_create: landing-page]
  ✓ Team created at ~/.octo-code/teams/landing-page/

[spawn_agent: layout]
  → Task: Create layout.tsx with navigation

[spawn_agent: hero]
  → Task: Create hero section with CTA

[spawn_agent: features]
  → Task: Create feature cards and footer

[check_inbox] Waiting for agents...
  ✓ layout@landing-page: "Done. Created app/layout.tsx"
  ✓ hero@landing-page: "Done. Created app/sections/Hero.tsx"
  ✓ features@landing-page: "Done. Created app/sections/Features.tsx"

[bash: npm run build]
  ✓ Build succeeded

[team_delete]
  ✓ Team cleaned up
```

### 파일 기반 조율

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

## 🔌 API 제공자

### Atlas Cloud (기본)

```bash
export ATLAS_API_KEY="sk-your-key"
octo-code

# 또는 여러 키 로드밸런싱
export ATLAS_API_KEYS="key1,key2,key3"
```

- 엔드포인트: `https://api.atlascloud.ai`
- 모든 모델에 단일 키로 접근
- Rate limit 자동 재시도

### OpenRouter

```bash
export OPENROUTER_API_KEY="sk-your-key"
octo-code --provider openrouter
```

- 엔드포인트: `https://openrouter.ai/api`
- 동일한 모델 세트 지원
- Pay-as-you-go 과금

---

## ⚙️ 설정

### 설정 파일 위치

| OS | 경로 |
|----|------|
| macOS | `~/Library/Application Support/octo-code/config.json` |
| Linux | `~/.config/octo-code/config.json` |

### 설정 파일 예시

```json
{
  "api_key": "sk-your-atlas-key",
  "api_keys": ["sk-key1", "sk-key2"],
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
```

### 환경변수

| 변수 | 설명 |
|------|------|
| `ATLAS_API_KEY` | Atlas Cloud API 키 |
| `ATLAS_API_KEYS` | 쉼표로 구분된 여러 키 |
| `OPENROUTER_API_KEY` | OpenRouter API 키 |
| `RUST_LOG` | 로그 레벨 (debug, info, warn) |

---

## 🧪 테스트

```bash
# 모든 테스트 실행
cargo test

# 상세 출력
cargo test -- --nocapture

# 특정 모듈 테스트
cargo test core::
cargo test tools::
cargo test storage::
```

### 테스트 커버리지

```
총 37개 테스트
├─ core: 18개 (메시지, 모델, 설정)
├─ tools: 16개 (파일, 팀, 태스크)
└─ storage: 4개 (SQLite CRUD)
```

---

## 📚 문서

- [아키텍처 문서 (한국어)](docs/architecture-ko.md)
- [아키텍처 문서 (English)](docs/architecture-en.md)
- [사용법 (한국어)](docs/usage-ko.md)
- [사용법 (English)](docs/usage-en.md)
- [기여 가이드](CONTRIBUTING.md)

---

## 🏭 배포 및 패키징

### 배포 채널

| 채널 | 명령어 | 사용처 |
|------|--------|--------|
| **crates.io** | `cargo install octo-code-agent` | Rust 사용자 (권장) |
| **GitHub Releases** | 바이너리 다운로드 | 일반 사용자 |
| **Docker Hub** | `docker pull johunsang/octo-code` | 컨테이너 환경 |
| **Homebrew** | `brew install octo-code` | macOS 사용자 (예정) |

### crates.io 배포

```bash
# 패키지 검증
cargo publish --dry-run

# 배포
cargo publish

# 확인
open https://crates.io/crates/octo-code-agent
```

**패키지 정보:**
- 이름: `octo-code-agent`
- 버전: `0.1.0`
- 바이너리: `octo-code`
- 라이브러리: `octo_code_agent`

### 프로젝트 구조 개선 (v0.1.0)

**이전 (Workspace):**
```
crates/
├── octo-core/          # 핵심 타입
├── octo-providers/     # API 제공자
├── octo-tools/         # 도구 구현
├── octo-agent/         # 에이전트 루프
├── octo-storage/       # SQLite 저장소
└── octo-cli/           # CLI 바이너리
```

**현재 (단일 Crate):**
```
src/
├── core/               # 통합된 핵심 모듈
├── providers/          # API 제공자
├── tools/              # 17개 도구
├── agent/              # 에이전트 루프
├── storage/            # SQLite 저장소
└── cli/                # CLI 인터페이스
```

**개선 사항:**
- ✅ 더 간단한 의존성 관리
- ✅ 더 빠른 컴파일
- ✅ 더 쉬운 배포 (`cargo install` 한 번으로 완료)
- ✅ 더 작은 바이너리 크기

---

## 🤝 기여

기여는 환영합니다! [CONTRIBUTING.md](CONTRIBUTING.md)를 참고해주세요.

### 개발 환경 설정

```bash
git clone https://github.com/johunsang/octo-code-agent
cd octo-code-agent
cargo build
cargo test
```

---

## 📝 라이선스

MIT License © 2025 [johunsang](https://github.com/johunsang)

---

## 🙏 감사

- [Zhipu AI](https://zhipu.ai/) - GLM 모델
- [DeepSeek](https://deepseek.com/) - DeepSeek V3.2
- [Alibaba Cloud](https://www.alibabacloud.com/) - Qwen3
- [Moonshot AI](https://moonshot.cn/) - Kimi
- [OpenRouter](https://openrouter.ai/) - API 게이트웨이
- [Atlas Cloud](https://atlascloud.ai/) - 통합 API

---

<p align="center">
  <b>Happy coding with Octo! 🐙</b>
</p>
