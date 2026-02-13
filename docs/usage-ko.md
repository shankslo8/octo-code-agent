# octo-code 사용법

**octo-code**는 터미널에서 작동하는 AI 코딩 어시스턴트 CLI 도구입니다. LLM(대형 언어 모델)을 활용해 코드 작성, 수정, 디버깅을 자율적으로 수행합니다.

---

## 📦 설치

### 사전 요구사항

- [Rust](https://rustup.rs/) 1.70 이상
- Atlas Cloud API 키

### 소스에서 설치

```bash
# 저장소 클론
git clone <repository-url>
cd octo-code-agent

# 설치
make install
# 또는
cargo install --path crates/octo-cli
```

### 자동 설치 스크립트

```bash
curl -fsSL https://example.com/install.sh | sh
```

---

## 🔑 초기 설정

### 1. API 키 설정

처음 실행 시 API 키를 입력하라는 메시지가 표시됩니다:

```bash
$ octo-code
🔑 Atlas Cloud API 키를 입력하세요: sk-...
✅ 설정이 ~/.config/octo-code/config.toml에 저장되었습니다.
```

### 2. 설정 파일 직접 작성

`~/.config/octo-code/config.toml` 파일을 직접 작성할 수도 있습니다:

```toml
[atlas]
api_key = "sk-your-api-key-here"

# 선택사항: 기본 모델 설정
[models]
default = "deepseek-ai/deepseek-v3.2-speciale"
coder = "zai-org/glm-5"
reasoning = "qwen/qwen3-max-2026-01-23"
```

---

## 🚀 기본 사용법

### 대화형 모드 (기본)

프롬프트 없이 실행하면 대화형 모드로 시작합니다:

```bash
$ octo-code
🐙 octo-code v0.1.0
💬 질문을 입력하세요 (quit: 종료, /help: 도움말)

> 이 프로젝트의 구조를 분석해줘
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
    --repl                    REPL 모드로 실행
    --tui                     TUI 모드로 실행
    --session <SESSION_ID>    이전 세션 재개
    --model <MODEL_ID>        사용할 모델 지정
    -h, --help                도움말 표시
    -V, --version             버전 표시
```

---

## 💬 대화 명령어

대화 중 사용할 수 있는 특수 명령어:

| 명령어 | 설명 |
|--------|------|
| `/quit`, `/q` | 종료 |
| `/help`, `/h` | 도움말 표시 |
| `/clear` | 화면 지우기 |
| `/sessions` | 저장된 세션 목록 |
| `/session <ID>` | 특정 세션 불러오기 |
| `/new` | 새 세션 시작 |

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
| `write` | ✅ 필요 | 파일 생성/쓰기 |
| `edit` | ✅ 필요 | 파일 수정 |
| `bash` | ✅ 위험 명령 | 셸 명령 실행 |
| `team_create` | ✅ 필요 | 팀 생성 |
| `spawn_agent` | ✅ 필요 | 에이전트 생성 |

**자동 승인되는 명령**: `ls`, `pwd`, `echo`, `cat`, `git status`, `git log` 등 안전한 명령

**확인 메시지 예시**:
```
⚠️  Permission requested: bash { command: "rm -rf target" }
Allow? [y]es / [n]o / [a]lways: 
```

---

## 💾 세션 관리

### 세션 저장

모든 대화는 자동으로 SQLite 데이터베이스에 저장됩니다.

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

```
> @team octo-code 기능-x-팀 "새로운 기능 구현"
```

### 작업 할당

```
> @task octo-code 기능-x-팀 "데이터베이스 스키마 설계"
> @task octo-code 기능-x-팀 "API 엔드포인트 구현"
> @task octo-code 기능-x-팀 "단위 테스트 작성"
```

### 작업 상태 확인

```
> @list octo-code 기능-x-팀
```

### 팀 삭제

```
> @delete octo-code 기능-x-팀
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

---

## 💰 비용 안내

Atlas Cloud를 통해 과금됩니다.

| 모델 | 입력 $/1M 토큰 | 출력 $/1M 토큰 |
|------|---------------|----------------|
| `deepseek-ai/deepseek-v3.2-speciale` | $0.27 | $0.41 |
| `zai-org/glm-5` | $0.80 | $2.56 |
| `moonshotai/kimi-k2.5` | $0.50 | $2.50 |
| `qwen/qwen3-max-2026-01-23` | $1.20 | $6.00 |

**비용 절약 팁**:
- `-p` 모드는 세션 없이 실행되어 히스토리 비용 감소
- 작은 작업에는 `Fast` 모델 사용
- 에이전트 루프는 반복할수록 입력 토큰이 누적됨

---

## 🔧 문제 해결

### API 키 오류

```
Error: Atlas API key not found
```

해결: `~/.config/octo-code/config.toml` 파일을 확인하세요.

### 빌드 실패

```bash
# 의존성 업데이트
cargo update

# 깨끗한 빌드
make clean && make build
```

### 데이터베이스 오류

```bash
# 데이터베이스 재초기화
rm ~/.local/share/octo-code/octo-code.db
```

---

## 📚 추가 자료

- [아키텍처 문서 (한국어)](architecture-ko.md)
- [아키텍처 문서 (English)](architecture-en.md)
- [GitHub Issues](https://github.com/your-repo/octo-code-agent/issues)

---

## 📝 라이선스

MIT License