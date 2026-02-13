# octo-code 사용법

**octo-code**는 터미널에서 작동하는 AI 코딩 어시스턴트 CLI 도구입니다. LLM(대형 언어 모델)을 활용해 코드 작성, 수정, 디버깅을 자율적으로 수행합니다.

---

## 📦 설치

### 사전 요구사항

- [Rust](https://rustup.rs/) 1.75 이상
- Atlas Cloud 또는 OpenRouter API 키

### 소스에서 설치

```bash
# 저장소 클론
git clone https://github.com/johunsang/octo-code-agent
cd octo-code-agent

# 설치
cargo install --path .

# 또는 릴리스 빌드
make install
```

---

## 🔑 초기 설정

### 1. API 키 설정

처음 실행 시 API 키를 입력하라는 메시지가 표시됩니다:

```bash
$ octo-code
🔑 Atlas Cloud API 키를 입력하세요 (입력 없이 Enter 시 OpenRouter): sk-...
✅ 설정이 저장되었습니다.
```

### 2. 설정 파일 직접 작성

`config.json` 파일을 직접 작성할 수도 있습니다:

**macOS:**
```bash
mkdir -p ~/Library/Application\ Support/octo-code
cat > ~/Library/Application\ Support/octo-code/config.json << 'EOF'
{
  "api_key": "sk-your-atlas-api-key",
  "api_keys": ["sk-your-atlas-api-key"],
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
EOF
```

**Linux:**
```bash
mkdir -p ~/.config/octo-code
cat > ~/.config/octo-code/config.json << 'EOF'
{
  "api_key": "sk-your-api-key",
  "provider_type": "atlas_cloud"
}
EOF
```

### 3. 환경변수 설정

```bash
# Atlas Cloud 사용
export ATLAS_API_KEY="sk-your-api-key"

# 또는 OpenRouter 사용
export OPENROUTER_API_KEY="sk-your-api-key"

# 여러 키 로드밸런싱
export ATLAS_API_KEYS="key1,key2,key3"
```

---

## 🚀 기본 사용법

### 대화형 모드 (기본)

프롬프트 없이 실행하면 대화형 모드로 시작합니다:

```bash
$ octo-code
🐙 octo-code v0.1.0

사용할 모델을 선택하세요:
1. GLM-5 (zai-org/glm-5) - $0.80/$2.56 per 1M tokens [기본]
2. GLM-4.7 (zai-org/glm-4.7) - $0.52/$1.75 per 1M tokens
3. DeepSeek V3.2 (deepseek-ai/deepseek-v3.2-speciale) - $0.26/$0.38 per 1M tokens
4. Qwen3 Max (qwen/qwen3-max-2026-01-23) - $1.20/$6.00 per 1M tokens
5. Qwen3 Coder (Qwen/Qwen3-Coder) - $0.78/$3.90 per 1M tokens
6. Kimi K2.5 (moonshotai/kimi-k2.5) - $0.50/$2.50 per 1M tokens

선택 (1-6, 기본: 1): 1

octo> 이 프로젝트의 구조를 분석해줘
🔍 파일을 탐색 중입니다...
...
```

### 한 번 실행 모드 (-p)

특정 프롬프트를 한 번만 실행합니다:

```bash
octo-code -p "버그를 고쳐줘"
octo-code --prompt "README.md 작성해줘"
```

### REPL 모드

```bash
octo-code --repl
```

### TUI 모드

터미널 UI에서 대화형으로 사용합니다:

```bash
octo-code --tui
```

---

## 📋 명령어 옵션

```
USAGE:
    octo-code [OPTIONS]

OPTIONS:
    -p, --prompt <PROMPT>     한 번 실행할 프롬프트
    -c, --cwd <PATH>          작업 디렉토리 지정
    -f, --output-format <FMT> 출력 형식 (text, json) [기본: text]
    -q, --quiet               진행 표시기 숨김
        --repl                REPL 모드로 실행
        --tui                 TUI 모드로 실행
        --session <SESSION_ID> 이전 세션 재개
    -m, --model <MODEL_ID>    사용할 모델 지정
        --provider <PROVIDER> API 제공자 (atlas, openrouter)
    -d, --debug               디버그 로그 활성화
    -h, --help                도움말 표시
    -V, --version             버전 표시
```

---

## 💬 대화 명령어

대화 중 사용할 수 있는 특수 명령어:

| 명령어 | 설명 |
|--------|------|
| `/quit`, `/q`, `exit` | 종료 |
| `/help`, `/h` | 도움말 표시 |
| `/clear` | 화면 지우기 |
| `/sessions` | 저장된 세션 목록 |
| `/session <ID>` | 특정 세션 불러오기 |
| `/new` | 새 세션 시작 |
| `/model` | 현재 모델 확인 |
| `/cost` | 토큰 사용량 및 비용 확인 |

---

## 🛠️ AI 도구 사용법

octo-code는 AI가 코드를 직접 조작할 수 있는 다양한 도구를 제공합니다.

### 파일 조회

```
> src/main.rs 파일 내용을 보여줘
```

AI가 자동으로 `view` 도구를 사용합니다:
```
📝 view: src/main.rs
```

### 파일 수정

```
> 37번 줄의 버그를 고쳐줘
```

AI가 `edit` 도구로 수정:
```
✏️ edit: src/main.rs (line 37)
```

### 파일 생성

```
> utils.rs 파일에 로깅 함수를 만들어줘
```

```
📝 write: src/utils.rs
```

### 명령 실행

```
> 테스트를 실행해줘
```

위험한 명령은 권한 확인:
```
⚠️  Permission requested: bash { command: "cargo test" }
Allow? [y]es / [n]o / [a]lways: y
🔧 bash: cargo test
```

### 코드 검색

```
> "TODO" 주석이 있는 파일을 찾아줘
```

```
🔍 grep: TODO
```

---

## 🔐 권한 시스템

일부 도구는 사용자 확인이 필요합니다:

| 도구 | 권한 필요 | 설명 |
|------|-----------|------|
| `view` | ❌ 없음 | 파일 읽기 |
| `ls` | ❌ 없음 | 디렉토리 목록 |
| `glob` | ❌ 없음 | 파일 패턴 검색 |
| `grep` | ❌ 없음 | 코드 검색 |
| `coderlm` | ❌ 없음 | 코드 인텔리전스 |
| `task_get` | ❌ 없음 | 작업 조회 |
| `task_list` | ❌ 없음 | 작업 목록 |
| `check_inbox` | ❌ 없음 | 메시지 수신 |
| `write` | ✅ 필요 | 파일 생성/쓰기 |
| `edit` | ✅ 필요 | 파일 수정 |
| `bash` | ✅ 위험 명령 | 셸 명령 실행 |
| `team_create` | ✅ 필요 | 팀 생성 |
| `team_delete` | ✅ 필요 | 팀 삭제 |
| `spawn_agent` | ✅ 필요 | 에이전트 생성 |
| `task_create` | ✅ 필요 | 작업 생성 |
| `task_update` | ✅ 필요 | 작업 업데이트 |
| `send_message` | ✅ 필요 | 메시지 전송 |

**자동 승인되는 명령**: `ls`, `pwd`, `echo`, `cat`, `git status`, `git log`, `git diff` 등 안전한 명령

**확인 메시지 예시**:
```
⚠️  Permission requested: bash { command: "rm -rf target" }
Allow? [y]es / [n]o / [a]lways: 
```

---

## 💾 세션 관리

### 세션 저장

모든 대화는 자동으로 SQLite 데이터베이스에 저장됩니다.

**데이터베이스 위치:**
- macOS: `~/Library/Application Support/octo-code/octo-code.db`
- Linux: `~/.local/share/octo-code/octo-code.db`

### 세션 목록 조회

```
> /sessions
```

출력 예시:
```
📋 저장된 세션:
   • sess_abc123 - "버그 수정" - 2026-02-13 10:30
   • sess_def456 - "리팩토링" - 2026-02-12 15:45
```

### 세션 재개

```bash
# 특정 세션 ID로 재개
octo-code --session sess_abc123
```

대화 중에도 세션 전환:
```
> /session sess_abc123
```

---

## 👥 팀 협업 (고급 기능)

여러 AI 에이전트를 병렬로 실행하여 복잡한 작업을 분할 처리합니다.

### 팀 생성

AI가 자동으로 `team_create` 도구를 사용합니다:

```
> Next.js 랜딩페이지를 만드는 팀을 구성해줘
```

```
[team_create: landing-page]
[spawn_agent: layout]    ← 레이아웃 + 네비게이션
[spawn_agent: hero]      ← 히어로 섹션 + CTA
[spawn_agent: features]  ← 피처 카드 + 푸터
```

### 태스크 관리

에이전트들은 파일 기반 태스크 보드로 조율됩니다:

```
~/.octo-code/
├── teams/{team-name}/
│   ├── config.json         # 팀 설정
│   └── inboxes/            # 에이전트별 메시지함
└── tasks/{team-name}/      # 태스크 보드
```

### 팀 삭제

```
> landing-page 팀을 삭제해줘
```

---

## 🎯 사용 예시

### 예시 1: 버그 수정

```bash
$ octo-code -p "src/parser.rs에서 파싱 에러를 고쳐줘"
```

AI의 동작:
1. 파일 읽기 (`view`)
2. 코드 분석
3. 수정 (`edit`)
4. 테스트 실행 (`bash`)

### 예시 2: 새 기능 추가

```bash
$ octo-code
> 사용자 인증 미들웨어를 추가해줘
```

### 예시 3: 코드 리뷰

```bash
$ octo-code -p "src/auth.rs 코드를 리뷰해줘"
```

### 예시 4: 문서 작성

```bash
$ octo-code -p "API 문서를 docs/api.md에 작성해줘"
```

### 예시 5: 리팩토링

```bash
$ octo-code -p "중복 코드를 제거하고 리팩토링해줘"
```

### 예시 6: 특정 모델 사용

```bash
$ octo-code -m "deepseek-ai/deepseek-v3.2-speciale" -p "코드를 최적화해줘"
```

### 예시 7: OpenRouter 사용

```bash
$ export OPENROUTER_API_KEY="sk-..."
$ octo-code --provider openrouter -p "코드 리뷰해줘"
```

---

## 💰 비용 안내

API 사용량에 따라 비용이 발생합니다.

| 모델 | 입력 $/1M 토큰 | 출력 $/1M 토큰 | 컨텍스트 |
|------|---------------|----------------|---------|
| `zai-org/glm-5` | $0.80 | $2.56 | 202K |
| `zai-org/glm-4.7` | $0.52 | $1.75 | 202K |
| `deepseek-ai/deepseek-v3.2-speciale` | $0.26 | $0.38 | 163K |
| `qwen/qwen3-max-2026-01-23` | $1.20 | $6.00 | 252K |
| `Qwen/Qwen3-Coder` | $0.78 | $3.90 | 262K |
| `moonshotai/kimi-k2.5` | $0.50 | $2.50 | 262K |

**비용 절약 팁**:
- `-p` 모드는 세션 없이 실행되어 히스토리 비용 감소
- 작은 작업에는 `GLM-4.7`이나 `DeepSeek V3.2` 사용
- 에이전트 루프는 반복할수록 입력 토큰이 누적됨
- `--quiet` 옵션으로 토큰 사용량 실시간 확인 가능

---

## 🔧 문제 해결

### API 키 오류

```
Error: No API key found
```

해결:
```bash
# 환경변수 설정 확인
export ATLAS_API_KEY="sk-your-key"

# 또는 설정 파일 확인
ls ~/Library/Application\ Support/octo-code/config.json  # macOS
ls ~/.config/octo-code/config.json                        # Linux
```

### 빌드 실패

```bash
# 의존성 업데이트
cargo update

# 깨끗한 빌드
cargo clean && cargo build --release
```

### 데이터베이스 오류

```bash
# 데이터베이스 재초기화
rm ~/Library/Application\ Support/octo-code/octo-code.db  # macOS
rm ~/.local/share/octo-code/octo-code.db                   # Linux
```

### Rate Limit 오류

```
Rate limited. Waiting 5s... (attempt 1/3)
```

이 메시지가 표시되면 자동으로 재시도합니다. 여러 API 키를 설정하여 로드밸런싱할 수 있습니다:

```bash
export ATLAS_API_KEYS="key1,key2,key3"
```

---

## 📚 추가 자료

- [아키텍처 문서 (한국어)](architecture-ko.md)
- [아키텍처 문서 (English)](architecture-en.md)
- [GitHub Issues](https://github.com/johunsang/octo-code-agent/issues)

---

## 📝 라이선스

MIT License
