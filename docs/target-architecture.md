# Target Architecture

## Status

이 문서는 기존 shell bootstrap과 ad-hoc 스크립트를 Rust 기반 워크스테이션 관리기와 `chezmoi` 파일 엔진으로 재구성하는 **최종 목표 아키텍처**를 정의한다.

이 문서는 greenfield target을 전제로 한다. 즉, legacy migration, backward compatibility, 기존 `stow` 사용자 전환, 기존 local file import 같은 요구는 설계 범위에 넣지 않는다.

## Problem Statement

현재 dotfiles 프로젝트는 다음 책임이 하나의 shell 중심 구조에 섞여 있다.

- machine-local state 해석
- optional feature 선택과 profile 적용
- OS/package-manager 분기
- tool 설치와 update orchestration
- `chezmoi` config 생성과 file apply
- post-apply 작업
- health check

이 구조는 기능이 늘수록 다음 문제가 커진다.

- shell quoting, exit code, partial failure 처리 난이도
- 상태 모델과 실행 모델의 분산
- 테스트 가능성 부족
- declarative data와 imperative code의 경계 불명확
- 새 기능 추가 시 bootstrap 스크립트 복잡도 급증

최종 구조의 목표는 이 책임들을 명확히 분리하고, 파일 렌더링은 `chezmoi`, 상태/정책/오케스트레이션은 Rust CLI가 담당하도록 만드는 것이다.

## Goals

- `chezmoi`를 최종 파일 엔진으로 유지한다.
- shell bootstrap 로직을 Rust CLI 하나로 통합한다.
- 새 머신에서 **clone 후 1-command bootstrap** 이 가능해야 한다.
- 기존 머신에서는 **저장된 local state를 기준으로 1-command update** 가 가능해야 한다.
- machine-local state와 generated snapshot을 분리한다.
- optional features, profiles, installers, package-manager mapping을 선언형 data로 관리한다.
- `latest-first` 정책을 명시적으로 지원한다.
- 상태 계산, diff, apply, doctor, tool sync를 테스트 가능한 도메인 로직으로 분리한다.
- `chezmoi`는 파일 관리만 담당하고, orchestration은 하지 않는다.

## Non-Goals

- legacy `stow` migration 지원
- `chezmoi run_*` 스크립트 기반 orchestration
- Home Manager/Nix style의 reproducible pinning
- 전체 시스템 설정 관리
- arbitrary shell snippet을 manifest에 내장하는 범용 task runner

## Design Principles

1. **One owner per concern**
   - 파일 렌더링과 target apply는 `chezmoi`가 담당한다.
   - 상태 계산, 설치 정책, 검사, 실행 순서는 Rust가 담당한다.

2. **Declarative first**
   - profile, optional feature, package mapping, installer metadata는 TOML로 선언한다.
   - 실행 중 조건 분기는 state resolution 결과를 기준으로 한다.

3. **Latest first**
   - 버전 고정보다 최신 안정 버전 동기화를 우선한다.
   - pinning은 upstream 제약이나 장애 회피가 필요한 경우에만 예외적으로 허용한다.

4. **No shell as business logic**
   - Rust가 직접 구현 가능한 로직은 shell로 두지 않는다.
   - 외부 package manager나 upstream installer는 Rust가 `Command`로 직접 호출한다.

5. **Idempotent operations**
   - `bootstrap`, `update`, `apply`, `doctor`는 반복 실행 가능해야 한다.
   - 각 단계는 현재 상태를 기준으로 수렴형 동작을 해야 한다.

6. **Human-readable local state**
   - local state는 사용자가 직접 읽고 편집할 수 있어야 한다.
   - generated files와 사용자 편집 파일은 분리한다.

## Final UX

### Fresh machine

권장 흐름은 아래 한 줄이다.

```sh
./install.sh
```

이 스크립트는 다음만 담당한다.

1. `dotctl` binary 확보
2. 현재 repo 경로를 `--repo`로 넘겨 `dotctl bootstrap` 실행

`dotctl bootstrap`은 local state가 없으면 interactive prompt 또는 CLI flags로 초기 state를 만든 뒤 전체 bootstrap을 완료한다.

### Existing machine

정기 업데이트의 기본 명령은 아래다.

```sh
dotctl update
```

이 명령은 저장된 local state를 기준으로 다음을 다시 맞춘다.

- package-manager installed packages
- user-local installed tools
- `chezmoi` managed files
- post-apply sync 대상
- health checks

### Narrow operations

```sh
dotctl diff
dotctl apply
dotctl doctor
dotctl state show
dotctl state edit
```

기본 원칙은 다음과 같다.

- 사용자는 일상적으로 `dotctl bootstrap` 또는 `dotctl update`를 사용한다.
- `chezmoi`는 내부 엔진이지만 file-only 작업이 필요할 때 직접 사용할 수 있다.
- `chezmoi update`는 지원되는 주 entrypoint가 아니다.

## Canonical Repository Layout

```text
.
├── .chezmoiroot
├── home/
│   ├── .chezmoiignore.tmpl
│   ├── dot_gitconfig.tmpl
│   ├── dot_zshenv
│   ├── dot_zprofile.tmpl
│   ├── dot_zshrc
│   └── dot_config/...
├── config/
│   ├── profiles/
│   │   ├── default.toml
│   │   ├── laptop.toml
│   │   └── minimal.toml
│   ├── bundles/
│   │   ├── core.toml
│   │   ├── dev.toml
│   │   ├── terminal.toml
│   │   ├── github.toml
│   │   ├── git-lfs.toml
│   │   ├── ai.toml
│   │   └── fastfetch.toml
│   └── installers.toml
├── crates/
│   ├── dotctl/
│   ├── dotctl-core/
│   └── dotctl-testkit/
├── install.sh
├── Cargo.toml
└── README.md
```

### Layout rationale

- `home/`는 `chezmoi` source-state root이다.
- `config/`는 Rust가 읽는 declarative control plane이다.
- `crates/`는 Rust implementation이다.
- `install.sh`만 장기적으로 남는 shell entrypoint이다.
- `scripts/`와 기존 bootstrap shell orchestration은 최종 구조에 존재하지 않는다. 공개 shell entrypoint는 root의 `install.sh` 하나만 남는다.

## State Model

### Repository-shared state

repo에는 machine-independent defaults만 저장한다.

- `config/profiles/*.toml`
- `config/bundles/*.toml`
- `config/installers.toml`
- `home/` templates and managed files

### Machine-local authoritative input

```text
~/.config/dotfiles/local.toml
```

이 파일은 사용자가 편집하는 authoritative machine-local state다.

예시:

```toml
profile = "default"

[repo]
source_dir = "/home/wonjae/dotfiles"

[features]
github = false
terminal_apps = true
git_lfs = true
ai_tools = false
fastfetch = false

[identity.git]
name = "Wonjae Choi"
email = "wonjae@example.com"
signing_key = ""
gpg_program = ""
sign_commits = false

[system]
package_manager_override = ""
```

원칙:

- local state는 sparse override가 아니라 **사용자가 의도적으로 선택한 machine contract** 를 담는다.
- profile 이름도 이 파일에 저장한다.
- repo path도 이 파일에 저장한다.
- secrets는 이 파일에 두지 않는다.

### Generated resolved snapshot

```text
~/.config/dotfiles/state.toml
```

이 파일은 `bootstrap` 또는 `update`가 성공했을 때만 기록되는 마지막 성공 실행의 resolved snapshot이다.

포함 내용:

- resolved feature flags
- selected profile
- detected platform
- chosen package manager
- resolved repo path
- applied bundle set
- tool versions if useful for diagnostics
- last applied timestamp

이 파일은 기록과 디버깅용이다. 다음 실행의 authoritative input은 아니다.

### Generated chezmoi config

```text
~/.config/chezmoi/chezmoi.toml
```

이 파일은 `dotctl`이 생성한다. 직접 수정 대상이 아니다.

권장 구조:

```toml
sourceDir = "/home/wonjae/dotfiles"

[data.repo]
source_dir = "/home/wonjae/dotfiles"

[data.features]
github = false
terminal_apps = true
git_lfs = true
ai_tools = false
fastfetch = false

[data.identity.git]
name = "Wonjae Choi"
email = "wonjae@example.com"
signing_key = ""
gpg_program = ""
sign_commits = false
```

### Local extension points

아래 파일은 unmanaged extension point로 남긴다.

- `~/.gitconfig.extra`
- `~/.zprofile.local`
- `~/.zshrc.local`

원칙:

- structured state로 표현 가능한 것은 local.toml로 올린다.
- truly ad-hoc local customization만 extension point에 둔다.
- `dotctl`은 extension point를 덮어쓰지 않는다.

## Command Model

```text
dotctl bootstrap
dotctl update
dotctl diff
dotctl apply
dotctl doctor
dotctl state show
dotctl state edit
dotctl features list
dotctl completion <shell>
```

### `dotctl bootstrap`

책임:

- repo path 결정
- local state 부재 시 초기 설정 생성
- platform/package manager 결정
- packages/tools 설치
- generated `chezmoi.toml` 작성
- `chezmoi apply`
- post-apply sync
- doctor 실행
- `state.toml` 기록

### `dotctl update`

책임:

- 저장된 local state 로드
- latest-first policy 기준으로 packages/tools refresh
- `chezmoi apply`
- post-apply sync
- doctor 실행
- snapshot 갱신

### `dotctl diff`

책임:

- local state를 기반으로 generated `chezmoi.toml` 갱신
- `chezmoi diff` 실행
- 필요한 경우 package/tool drift summary도 함께 출력

### `dotctl apply`

책임:

- file management만 수행
- package install/update는 하지 않음
- `chezmoi apply` + 최소 post-apply만 수행

### `dotctl doctor`

책임:

- required tool presence
- generated config sanity
- `chezmoi verify`
- zsh bundle/nvim/yazi state smoke check
- broken local assumptions 탐지

## Execution Model

최종 구현은 imperative shell sequence가 아니라 **operation graph** 로 구성한다.

핵심 operation:

- `ResolveState`
- `GenerateChezmoiConfig`
- `DetectPlatform`
- `SelectPackageManager`
- `InstallPackages`
- `InstallManagedTools`
- `ApplyChezmoi`
- `SyncZshPlugins`
- `SyncYaziPackages`
- `SyncNeovim`
- `RunDoctor`
- `WriteSnapshot`

각 command는 operation subset을 사용한다.

| Command | Operations |
| --- | --- |
| `bootstrap` | 전체 |
| `update` | 전체에서 prompt/state-init 제외 |
| `apply` | `ResolveState`, `GenerateChezmoiConfig`, `ApplyChezmoi`, 최소 post-apply |
| `diff` | `ResolveState`, `GenerateChezmoiConfig`, diff/report only |
| `doctor` | verify/smoke only |

원칙:

- 각 operation은 idempotent 하다.
- operation은 typed input/output을 갖는다.
- operation ordering은 runner가 담당한다.
- 실패 시 snapshot은 쓰지 않는다.

## Chezmoi Contract

`chezmoi`는 이 시스템에서 다음 역할만 가진다.

- source-state를 target file set으로 렌더링
- machine-local data를 template input으로 사용
- target diff/apply/status/verify 제공

`chezmoi`가 맡지 않는 것:

- package/tool installation policy
- feature resolution
- profile merge
- health checks
- workstation lifecycle command UX

### Rules

1. `home/` 아래에는 `chezmoi` source-state만 둔다.
2. 핵심 orchestration을 `run_*` scripts로 구현하지 않는다.
3. optional file inclusion은 `.chezmoiignore.tmpl`과 template condition으로만 처리한다.
4. generated artifact는 repo에 두지 않는다.
5. `dotctl`은 managed target files를 직접 편집하지 않고 `chezmoi`를 통해 적용한다.

## Feature and Bundle Model

최종 구조에서 feature와 bundle은 구분한다.

- **feature**: 사용자 선택 단위
- **bundle**: 실제 설치/적용 단위

예시:

- feature `terminal_apps`
  - bundle `terminal`
  - file gating: `tmux`, `btop`, `starship`, `yazi`
  - tool sync: `starship`, `yazi`
- feature `git_lfs`
  - bundle `git-lfs`
  - package install: `git-lfs`
  - file rendering: `~/.gitconfig` 내 LFS section

### Profile

profile은 feature defaults와 일부 system defaults를 제공한다.

예시:

```toml
# config/profiles/default.toml
[features]
github = false
terminal_apps = true
git_lfs = true
ai_tools = false
fastfetch = false
```

### Bundle

예시:

```toml
# config/bundles/terminal.toml
id = "terminal"

[feature]
key = "terminal_apps"

[packages.brew]
names = ["tmux", "btop"]

[packages.apt]
names = ["tmux", "btop"]

[packages.dnf]
names = ["tmux", "btop"]

[packages.pacman]
names = ["tmux", "btop"]

[tools]
post_apply = ["starship", "yazi"]

[files]
chezmoi_feature_flag = "terminal_apps"
```

원칙:

- feature는 UX와 state 관점의 이름이다.
- bundle은 installer/file/doctor 관점의 실제 실행 단위다.
- 하나의 feature가 여러 bundle을 활성화할 수 있다.

## Installer and Package Architecture

### Package managers

OS package manager는 trait 뒤로 숨긴다.

```rust
trait PackageManager {
    fn id(&self) -> PackageManagerId;
    fn install(&self, packages: &[String]) -> Result<()>;
    fn upgrade_managed(&self, packages: &[String]) -> Result<()>;
    fn is_available(&self) -> bool;
}
```

지원 대상:

- Homebrew
- apt
- dnf
- pacman

원칙:

- `dotctl`은 package manager를 직접 shell script로 감싸지 않는다.
- distro-wide full system upgrade는 자동으로 하지 않는다.
- managed package subset만 설치/업데이트한다.

### Managed tools

OS package manager 외 도구는 installer registry로 관리한다.

예시 대상:

- Neovim
- starship
- yazi
- uv
- rustup
- AI CLIs

installer metadata는 `config/installers.toml`에 선언하고, managed tool install/update 경로의 단일 소스 오브 트루스로 사용한다.

예시:

```toml
[installers.neovim]
kind = "github_release"
repo = "neovim/neovim"
channel = "stable"
verify_checksum = true

[installers.starship]
kind = "script"
url = "https://starship.rs/install.sh"
update_policy = "latest"
```

원칙:

- 가능한 경우 package manager를 우선한다.
- package manager에 없거나 user-local install이 더 적절한 도구만 custom installer를 쓴다.
- `curl | sh`는 금지한다.
- script installer는 temp file download 후 명시적으로 실행한다.
- upstream이 checksum/signature를 제공하면 검증한다.

## Post-Apply Tasks

최종 구조에서는 post-apply를 shell hook가 아니라 named Rust task로 다룬다.

대표 task:

- `zsh_bundle`
- `yazi_sync`
- `nvim_sync`

원칙:

- post-apply task는 feature/bundle에 의해 활성화된다.
- task는 dotfile apply 이후에만 실행된다.
- task는 target-side user config를 source of truth로 읽는다.
- task 결과가 file drift를 만들면 그 drift는 repo가 아니라 user-local state/cache에만 생겨야 한다.

## Rust Workspace and Module Design

### Workspace

```text
crates/
├── dotctl/
├── dotctl-core/
└── dotctl-testkit/
```

### `dotctl`

역할:

- CLI parsing
- user-facing output formatting
- process exit semantics
- shell completion generation

권장 라이브러리:

- `clap`
- `miette`

### `dotctl-core`

역할:

- state loading and validation
- profile/bundle/installers parsing
- resolve engine
- operation graph runner
- package manager adapters
- `chezmoi` integration
- doctor checks
- reporting model

권장 모듈:

- `cli_contract`
- `state`
- `manifest`
- `platform`
- `packages`
- `installers`
- `chezmoi`
- `tasks`
- `doctor`
- `report`
- `runner`

권장 라이브러리:

- `serde`
- `toml`
- `tracing`
- `tracing-subscriber`
- `miette`

### `dotctl-testkit`

역할:

- temp HOME fixture
- fake package manager binaries
- fake upstream downloads
- golden file rendering tests
- cross-platform scenario harness

이 crate는 필수는 아니지만, 장기적으로 CLI와 orchestration 품질을 지키려면 분리하는 편이 좋다.

## Observability and Error Model

### Logging

- 기본 출력은 concise human-readable log
- `--verbose`는 step-level detail
- `--json`은 automation-friendly structured events
- 모든 long-running step은 start/end/failure를 명확히 남긴다

### Errors

오류는 다음 기준으로 분류한다.

- user configuration error
- platform/package-manager error
- external command failure
- rendering/apply error
- verification error

원칙:

- partial failure를 성공처럼 숨기지 않는다.
- exit code는 command contract에 맞게 안정적이어야 한다.
- doctor는 failure summary와 fix hint를 함께 출력한다.

## Security Model

- repo에는 secrets를 저장하지 않는다.
- local state에는 identity와 machine-local selection만 저장한다.
- secret material은 `chezmoi` template functions와 password manager integration, 또는 unmanaged private local files로 처리한다.
- generated local state와 generated `chezmoi.toml`은 최소 `0600` 권한을 권장한다.
- 원격 script 실행은 explicit download + explicit exec만 허용한다.

## Testing Strategy

### Unit tests

- profile merge
- feature resolution
- bundle selection
- package-manager selection
- generated `chezmoi.toml` rendering
- installer argument construction

### Integration tests

- temp HOME에서 `bootstrap`
- existing local state 기반 `update`
- `apply` only path
- feature on/off에 따른 `chezmoi managed` 결과
- `doctor` success/failure cases

### Golden tests

- rendered `~/.gitconfig`
- rendered `~/.zprofile`
- `.chezmoiignore` gating
- generated `chezmoi.toml`

### Real command smoke tests

- `chezmoi apply`
- `chezmoi verify`
- `zsh -ic`
- `nvim --headless '+qall'`
- `ya pkg install/upgrade` selection path

## Why This Is the Target Design

이 구조를 최종 목표로 삼는 이유는 다음과 같다.

- 파일 관리 엔진은 이미 `chezmoi`가 충분히 잘한다.
- 현재 복잡성의 핵심은 file placement가 아니라 state, policy, orchestration이다.
- 따라서 `chezmoi`를 대체하는 것보다 Rust가 `chezmoi`를 감싸는 구조가 더 단순하다.
- Rust는 shell보다 state resolution, error propagation, tests, long-term maintenance에 유리하다.
- 선언형 config와 typed execution graph를 도입하면 optional feature와 installer가 늘어나도 구조가 무너지지 않는다.

## Explicit Rejections

아래 대안은 최종 구조로 채택하지 않는다.

### 1. Pure shell orchestration 유지

이유:

- 상태와 정책이 커질수록 유지보수성이 급격히 나빠진다.
- quoting, exit handling, testability 비용이 너무 높다.

### 2. `chezmoi run_*` script 중심 구조

이유:

- orchestration 로직이 다시 shell로 분산된다.
- file engine과 lifecycle engine의 경계가 흐려진다.

### 3. `stow` 또는 custom symlink engine 복귀

이유:

- machine-local rendering과 optional file gating을 다시 직접 풀어야 한다.
- 이미 `chezmoi`가 더 적합한 문제를 잘 해결한다.

### 4. Full reproducible pinning-first model

이유:

- 이 프로젝트의 목표는 latest-first workstation sync다.
- pinning은 기본 정책이 아니라 예외 정책이어야 한다.

## References

- chezmoi configuration file: <https://www.chezmoi.io/reference/configuration-file/>
- chezmoi source state attributes: <https://www.chezmoi.io/reference/source-state-attributes/>
- chezmoi customize source directory: <https://www.chezmoi.io/user-guide/advanced/customize-your-source-directory/>
- chezmoi machine-to-machine differences: <https://www.chezmoi.io/user-guide/manage-machine-to-machine-differences/>
- clap derive tutorial: <https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html>
- serde attributes: <https://serde.rs/attributes.html>
- tracing crate docs: <https://docs.rs/tracing/>
- miette crate docs: <https://docs.rs/miette>
