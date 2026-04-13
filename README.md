# dotfiles

Personal workstation configuration for `wonjae`, managed with `GNU Stow`.

This repo stores the shared, reproducible part of the environment. Identity, secrets, credentials, and host-specific paths stay in local files under `$HOME` and are never committed.

## Quick Start

1. Clone this repo to `~/dotfiles`.
2. Copy `templates/gitconfig.local` to `~/.gitconfig.local` and fill in Git identity.
3. Optionally copy `templates/zprofile.local` and `templates/zshrc.local`.
4. Run `./bootstrap/install.sh`.
5. On development machines, optionally run `./bootstrap/install-dev.sh`.
6. Run `./scripts/check`.
7. Run `./scripts/stow`.
8. Open a new login shell and verify the environment.

## Daily Use

Preview changes:

```sh
./scripts/check
```

Apply symlinks:

```sh
./scripts/stow
```

Rebuild symlinks:

```sh
./scripts/restow
```

Remove managed symlinks:

```sh
./scripts/unstow
```

Operate on specific packages:

```sh
./scripts/check shell zsh git
./scripts/stow git starship nvim
./scripts/restow zsh tmux
./scripts/unstow gh
```

Test against a temporary target:

```sh
TARGET_DIR=/tmp/dotfiles-test ./scripts/check
```

## Layout

- `shell/`, `zsh/`, `tmux/`, `git/`, `starship/`, `nvim/`, `gh/`, `btop/`: stowed config packages
- `bootstrap/`: package installation scripts and manifests
- `scripts/`: thin wrappers around `stow`
- `templates/`: starter files for local-only config
- `fastfetch/`: saved config only, not applied by default

## Local-Only Files

These are intentionally not tracked:

- `~/.gitconfig.local`
- `~/.zprofile.local`
- `~/.zshrc.local`
- `~/.config/gh/hosts.yml`
- secrets and credentials under `~/.ssh`, `~/.aws`, `~/.gnupg`, `~/.codex`, and similar directories
- generated files such as `~/.zsh_plugins.zsh` and `~/.cache/zsh/zcompdump-*`
- installed binaries and caches under `~/.local`, `~/.cargo`, `~/.rustup`, and `~/.cache`

## Shell Model

Shared shell helpers live under `shell/.config/shell/`.

- `env.sh`: common exported defaults such as `EDITOR`, `PAGER`, and `GOPATH`
- `path.sh`: PATH cleanup, shared tool PATH entries, and optional env scripts such as cargo
- `zsh/.zshenv`: minimal environment for every zsh process
- `zsh/.zprofile`: login-shell setup, then `~/.zprofile.local`
- `zsh/.zshrc`: interactive behavior, plugins, aliases, prompt, then `~/.zshrc.local`

## Bootstrap

`bootstrap/` installs external tools. `stow` manages symlinks.

- `./bootstrap/install.sh`: core toolchain, recommended packages, local `git-lfs` setup when present, antidote, then a post-check
- `./bootstrap/install.sh --required-only`: install only the minimum needed to manage this repo
- `./bootstrap/install-dev.sh`: development tooling such as `node`, `npm`, `uv`, `rustup`, and `codex`
- supported package managers: Homebrew on macOS, `apt`/`dnf`/`pacman` on Linux
- manifest files under `bootstrap/manifests/` are the source of truth for package-manager-specific packages
- on Linux, system package installation uses `sudo` and recommended packages are best-effort
- `git-lfs` filters are written into `~/.gitconfig.local`, not the shared repo-managed Git config

## Migration Notes

If a target path already exists as a regular file, `stow` will refuse to overwrite it.

Typical migration flow:

1. Run `./bootstrap/install.sh`.
2. On development machines, optionally run `./bootstrap/install-dev.sh`.
3. Run `./scripts/check` and inspect conflicts.
4. Move conflicting files into a backup directory under `$HOME`.
5. Copy identity or host-specific settings into the appropriate `*.local` files.
6. Run `./scripts/stow`.
7. Open a new login shell and verify `zsh`, `git`, and `PATH`.
