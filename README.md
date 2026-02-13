# OctoCode Agent 🐙

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**AI 코딩 에이전트** — 여러 에이전트가 동시에 코드를 분석하고 수정하는 터미널 기반 코딩 어시스턴트.

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

## 주요 기능

- **병렬 멀티 에이전트** — 작업을 자동 분해하여 여러 에이전트가 동시에 작업
- **팀 조율 시스템** — 파일 기반 태스크 보드, 인박스 메시징, 자동 스폰
- **CodeRLM 통합** — tree-sitter 기반 코드 인텔리전스 (선택사항, 없어도 동작)
- **7개 모델 지원** — GLM 5, GLM 4.7, DeepSeek V3.2, Qwen3 Max, Qwen3 Coder, Kimi K2 Thinking, MiniMax M2.5
- **비용 추적** — 실시간 토큰 사용량 + 원화(₩) 변환 표시
- **세션 관리** — SQLite 기반 대화 히스토리, 세션 이어하기
- **Rate Limit 대응** — 자동 재시도, 지수 백오프, 에이전트 stagger 스폰

## 설치

```bash
# 소스에서 빌드 (Rust 1.75+ 필요)
git clone https://github.com/anthropics/octo-code-agent
cd octo-code-agent
cargo build --release

# 설치
cargo install --path crates/octo-cli

# 또는 직접 복사
cp target/release/octo-code ~/.local/bin/
```

### 원라인 설치

```bash
curl -fsSL https://raw.githubusercontent.com/anthropics/octo-code-agent/main/install.sh | bash
```

## API 키 설정

Atlas Cloud API 키 하나로 모든 모델을 사용합니다.

```bash
# 방법 1: 환경변수 (권장)
export ATLAS_API_KEY="your-key-here"

# 방법 2: 처음 실행 시 자동 셋업 화면에서 입력
octo-code

# 방법 3: 설정 파일 직접 편집
# macOS: ~/Library/Application Support/octo-code/config.json
# Linux: ~/.config/octo-code/config.json
```

```json
{
  "api_key": "your-key-here",
  "base_url": "https://api.atlascloud.ai"
}
```

## 사용법

```bash
# 인터랙티브 모드 (기본) — 모델 선택 → 작업 입력
octo-code

# 단일 명령 모드
octo-code -p "main.rs의 버그를 고쳐줘"

# 모델 지정
octo-code -m "zai-org/glm-5"

# REPL 모드
octo-code --repl

# TUI 모드
octo-code --tui

# 세션 이어하기
octo-code --session <session_id>

# JSON 출력
octo-code -p "설명해줘" -f json

# 디버그 로그
octo-code -d
```

### 인터랙티브 명령어

| 명령어 | 설명 |
|--------|------|
| `/help` | 도움말 |
| `/model` | 현재 모델 확인 |
| `/cost` | 토큰 사용량 + 비용 |
| `/sessions` | 세션 목록 |
| `/clear` | 세션 초기화 |
| `/exit` | 종료 |

## 지원 모델

| 모델 | 벤더 | 입력 ($/M) | 출력 ($/M) | 컨텍스트 | 특징 |
|------|-------|-----------|-----------|---------|------|
| **GLM 5** | Zhipu AI | $0.80 | $2.56 | 202K | 745B MoE, 기본 모델 |
| **GLM 4.7** | Zhipu AI | $0.52 | $1.75 | 202K | 경제적, 131K output |
| **DeepSeek V3.2** | DeepSeek | $0.26 | $0.38 | 163K | 685B MoE, 최저가 |
| **Qwen3 Max** | Alibaba | $1.20 | $6.00 | 252K | Flagship reasoning |
| **Qwen3 Coder** | Alibaba | $0.78 | $3.80 | 262K | 480B MoE, 코드 특화 |
| **Kimi K2 Thinking** | Moonshot | $0.60 | $2.50 | 262K | Deep reasoning |
| **MiniMax M2.5** | MiniMax | $0.29 | $0.95 | 196K | 경량, 빠른 응답 |

## 아키텍처

```
octo-code-agent/
├── crates/
│   ├── octo-core/       # 핵심 타입, 설정, 에러, 모델 정의
│   ├── octo-providers/  # LLM API 프로바이더 (OpenAI-compatible)
│   ├── octo-tools/      # 도구 구현 (17개)
│   ├── octo-agent/      # 에이전트 루프, 프롬프트, 스트리밍
│   ├── octo-storage/    # SQLite 세션/메시지 저장
│   └── octo-cli/        # CLI 바이너리, 인터랙티브 모드, TUI
├── install.sh           # 원라인 설치 스크립트
├── Makefile             # 빌드/배포 타겟
└── .github/workflows/   # CI/CD (테스트, 릴리스 빌드)
```

### 도구 목록

| 도구 | 설명 |
|------|------|
| `bash` | 셸 명령 실행 |
| `view` | 파일 읽기 |
| `write` | 파일 생성 |
| `edit` | 파일 수정 (문자열 치환) |
| `ls` | 디렉토리 목록 |
| `glob` | 패턴으로 파일 검색 |
| `grep` | 정규식으로 코드 검색 |
| `coderlm` | 코드 인텔리전스 (선택사항) |
| `team_create` | 팀 생성 |
| `team_delete` | 팀 삭제 |
| `spawn_agent` | 에이전트 스폰 |
| `task_create` | 태스크 생성 |
| `task_get` | 태스크 조회 |
| `task_update` | 태스크 업데이트 |
| `task_list` | 태스크 목록 |
| `send_message` | 메시지 전송 |
| `check_inbox` | 메시지 수신 |

### 병렬 처리 흐름

```
사용자 요청
    │
    ▼
┌─────────────┐
│  리드 에이전트  │ ← coderlm/grep으로 코드 분석
│  (Team Lead)  │
└──────┬──────┘
       │ spawn_agent × N
       ├──────────────┬──────────────┐
       ▼              ▼              ▼
  ┌─────────┐   ┌─────────┐   ┌─────────┐
  │ Agent 1 │   │ Agent 2 │   │ Agent 3 │
  │ (impl)  │   │ (tests) │   │ (docs)  │
  └────┬────┘   └────┬────┘   └────┬────┘
       │              │              │
       └──────────────┴──────────────┘
                      │ send_message → check_inbox
                      ▼
               ┌─────────────┐
               │  리드 에이전트  │ ← 빌드/테스트 검증
               │  결과 통합     │
               └─────────────┘
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

## CodeRLM (선택사항)

[CodeRLM](https://github.com/JaredStewart/coderlm) 서버가 실행 중이면 자동 감지하여 tree-sitter 기반 코드 인텔리전스를 사용합니다. 없으면 `grep`/`glob`/`view`로 대체합니다.

```bash
# CodeRLM 서버 실행 (선택사항)
cd coderlm/server && npm start
# → http://127.0.0.1:9999 에서 실행

# octo-code 실행 시 자동 감지
octo-code
# ✓ CodeRLM connected  ← 연결 성공 시 표시
```

## 개발

```bash
# 빌드
cargo build

# 테스트
cargo test --workspace

# 릴리스 빌드
cargo build --release

# Clippy
cargo clippy --workspace

# 포맷
cargo fmt --all
```

## 라이선스

MIT License

---

**Happy coding with Octo!** 🐙
