# dotfiles

Personal workstation configuration for `wonjae`, managed with `GNU Stow`.

This repository is the source of truth for shared, reproducible configuration. Machine-specific identity, secrets, and one-off paths stay in local files under `$HOME` and are never committed.

## Design Principles

- Repo-managed files should be safe to sync across machines.
- Prefer XDG-style locations under `~/.config` when the tool supports them.
- Keep shell startup layered:
  - `~/.zshenv`: minimal environment for every zsh process
  - `~/.zprofile`: login-shell environment
  - `~/.zshrc`: interactive shell behavior
- Keep `zsh` as the primary shell.
- Local identity, signing keys, and machine-specific overrides belong in `*.local` files.

## Layout

- `shell/`: shared login-shell helper scripts for environment and PATH cleanup
- `bootstrap/`: install scripts and package manifests for external tools
- `zsh/`: primary zsh startup files and plugin list
- `tmux/`: tmux configuration
- `git/`: shared Git config and global ignore rules
- `starship/`: prompt configuration under `~/.config`
- `nvim/`: editor configuration
- `gh/`: GitHub CLI defaults without credentials
- `btop/`: terminal system monitor configuration
- `fastfetch/`: system summary configuration
- `templates/`: local-only starter files that should not be stowed
- `scripts/`: thin wrappers around `stow`

## Shell Startup Model

Shared login-shell helpers live under `shell/.config/shell/`.

- `env.sh`: common exported defaults such as `EDITOR`, `PAGER`, and `GOPATH`
- `path.sh`: PATH cleanup, shared tool PATH entries, and optional tool env scripts

`path.sh` is responsible for:

- cleaning stale PATH entries
- adding common tool locations only when they exist
- loading optional shared env scripts such as cargo
- deduplicating PATH after all shared setup

Then each shell entrypoint stays focused on its own role:

- `zsh/.zshenv`: minimal environment for every zsh invocation
- `zsh/.zprofile`: source the shared login helpers, then apply zsh-local login overrides
- `zsh/.zshrc`: interactive zsh features, plugins, prompt, aliases, and hooks

Local file split:

- `~/.zprofile.local`: exported environment variables, package-manager bootstrap, and PATH changes that should be in place before the shared `~/.zshrc` runs
- `~/.zshrc.local`: interactive-only customizations such as aliases, shell hooks, feature toggles, and terminal-specific behavior

## Local-Only Files

These are intentionally not tracked:

- `~/.gitconfig.local`
- `~/.zprofile.local`
- `~/.zshrc.local`
- `~/.config/gh/hosts.yml`
- secrets and credentials under `~/.ssh`, `~/.aws`, `~/.gnupg`, `~/.codex`, and similar directories
- generated files such as `~/.zsh_plugins.zsh` and `~/.cache/zsh/zcompdump-*`
- installed binaries and caches under `~/.local`, `~/.cargo`, `~/.rustup`, and `~/.cache`

## First-Time Setup

On a fresh machine:

- Bootstrap expects a supported system package manager:
  - macOS with Homebrew already installed
  - Linux with `apt`, `dnf`, or `pacman`

1. Clone this repository to `~/dotfiles`.
2. Copy `templates/gitconfig.local` to `~/.gitconfig.local` and fill in your identity.
3. Optionally copy any of these templates if needed:
   `templates/zprofile.local`, `templates/zshrc.local`
4. Run `./bootstrap/install.sh`.
5. On development machines, optionally run `./bootstrap/install-dev.sh`.
6. Run `./scripts/check`.
7. Run `./scripts/stow`.
8. Open a new login shell and verify the environment.

## Existing Machine Migration

If a target path already exists as a regular file, `stow` will refuse to overwrite it.

Typical migration flow:

1. Run `./bootstrap/install.sh`.
2. On development machines, optionally run `./bootstrap/install-dev.sh`.
3. Run `./scripts/check` and inspect conflicts.
4. Move conflicting files into a backup directory under `$HOME`.
5. Copy any identity or machine-specific settings into the appropriate `*.local` files.
6. Run `./scripts/stow`.
7. Open a new login shell and verify `zsh`, `git`, and `PATH`.

## Bootstrap

Use bootstrap scripts for external tools. Keep package installation separate from `stow`.

- `./bootstrap/install.sh`: install required tools, install recommended tools when available, ensure a managed `brew shellenv` block in `~/.zprofile.local` when bootstrap runs with `brew`, initialize `git-lfs` when present, then install antidote and run a post-check
- `./bootstrap/install.sh --required-only`: install only the minimum toolchain for managing the repo and skip antidote
- `./bootstrap/install.sh --package-manager=brew|apt|dnf|pacman`: override package-manager detection
- `./bootstrap/install-dev.sh`: install development-only tooling outside `stow`: `node`, `npm`, `uv`, `rustup`, and `Codex`
- `./bootstrap/install-dev.sh --package-manager=brew|apt|dnf|pacman`: override package-manager detection for development prerequisites

Top-level `bootstrap/` is intentionally small:

- entrypoints: `install.sh`, `install-dev.sh`
- shared helpers: `bootstrap/lib/`
- internal helpers and post-install checks: `bootstrap/internal/`
- package manifests: `bootstrap/manifests/`

Bootstrap policy:

- required tools: `git`, `stow`, `zsh`, `curl`
- recommended tools: `git-lfs`, `tmux`, `neovim`, `starship`, `gh`, `fzf`, `zoxide`, `eza`, `btop`, `fastfetch`, `yazi`
- development tools: `node`, `npm`, `uv` for Python workflows, `rustup` for Rust, and `Codex`

On Linux, the install script uses `sudo` for system package installation. Recommended packages are installed on a best-effort basis so distro-specific package gaps do not abort the whole bootstrap.

`install-dev.sh` keeps development tooling in the user account instead of mixing it into the shared dotfiles:

- `node` and `npm` are installed with the system package manager as part of the development baseline
- `uv` is installed with its standalone installer and is expected to manage Python versions on demand
- `rustup` is installed under `~/.cargo` without modifying shell startup files
- `Codex` is installed with `npm` using `~/.local` as the global prefix, so it does not need `sudo`
- `Claude` is intentionally left to manual installation because the official installer modifies shell startup files outside this repo's control

## Scripts

- `./scripts/check`: dry-run and local-file checks
- `./scripts/stow`: apply packages
- `./scripts/restow`: restow packages
- `./scripts/unstow`: remove managed symlinks

Each script also accepts package names. For example:

```sh
./scripts/check shell zsh git
./scripts/stow git starship nvim
./scripts/restow zsh tmux
./scripts/unstow fastfetch
```

The wrappers also respect `TARGET_DIR`, which is useful for testing against a temporary home directory.

```sh
TARGET_DIR=/tmp/dotfiles-test ./scripts/check
```

## Notes

- `git` includes `~/.gitconfig.local` for identity and signing configuration.
- `zsh` is the primary shell and owns the real shell environment model for this repo.
- `zsh` loads `~/.zprofile.local` and `~/.zshrc.local` if present.
- package-manager bootstrap such as Homebrew or Linuxbrew belongs in `~/.zprofile.local`, not in the shared repo-managed config
- on brew-based machines, `bootstrap/install.sh` maintains a managed `brew shellenv` block inside `~/.zprofile.local`
- `git-lfs` is not forced from the shared Git config. If it is installed, bootstrap initializes it with `git lfs install`.
- Install external tools referenced by the configs such as `antidote`, `fzf`, `zoxide`, `starship`, `tmux`, `nvim`, and `gh`.
