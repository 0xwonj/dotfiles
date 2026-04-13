# dotfiles

Personal workstation configuration for `wonjae`, managed with `GNU Stow`.

This repo stores the shared, reproducible part of the environment. Identity, secrets, credentials, and host-specific paths stay in local files under `$HOME` and are never committed.

## Quick Start

1. Clone this repo to `~/dotfiles`.
2. Copy `templates/gitconfig.local` to `~/.gitconfig.local` and fill in Git identity.
3. Optionally copy `templates/zprofile.local` and `templates/zshrc.local`.
4. On macOS, run `xcode-select --install` once before bootstrapping. On Arch/pacman, run `sudo pacman -Syu`.
5. Run:

```sh
./bootstrap/workstation.sh
```

This installs the baseline workstation: core tools, baseline developer tooling, the default stowed dotfiles, and the Neovim environment.

Optional groups:

- `--with-github`: `gh` CLI
- `--with-terminal-apps`: `tmux`, `btop`, plus user-local installs of `starship` and `yazi`, and the stowed `tmux/`, `btop/`, and `starship/` packages
- `--with-git-lfs`: `git-lfs` and local LFS filters in `~/.gitconfig.local`
- `--with-ai-tools`: `codex` and `claude`
- `--with-all-optional`: all of the above

Example:

```sh
./bootstrap/workstation.sh --with-github --with-terminal-apps
```

Routine refresh on an already-provisioned machine:

```sh
./bootstrap/workstation.sh --update
```

## Layout

- `shell/`, `zsh/`, `git/`, `nvim/`: default stowed baseline packages
- `tmux/`, `btop/`, `starship/`: opt-in stowed packages used by workstation flags
- `fastfetch/`: manual-only config package, not applied by default
- `bootstrap/`: installation and provisioning scripts
- `scripts/`: thin wrappers around `stow`
- `templates/`: starter files for local-only config

## Notes

- Run bootstrap scripts as your normal user, not with `sudo`. If needed, authenticate first with `sudo -v`.
- `./bootstrap/workstation.sh` is the reproducible bootstrap path.
- `./bootstrap/workstation.sh --update` is the explicit maintenance path.
- Both commands are intended to be safe to rerun.
- Neovim is installed from the latest official stable GitHub release into versioned directories under `~/.local/opt`, with `~/.local/opt/nvim-stable` and `~/.local/bin/nvim` repointed atomically.
- Neovim plugins are restored from `nvim/.config/nvim/lazy-lock.json` during bootstrap.
- `fzf`, `zoxide`, and `eza` are installed by default as convenience tools; missing them does not fail bootstrap.
- `--with-ai-tools` is opt-in because these CLIs are account-scoped local tools, and Claude Code's official install flow may still prompt shell `PATH` guidance around `~/.local/bin`.
- Supported target environments: macOS, Ubuntu, and Arch Linux.
- Supported package managers: Homebrew on macOS, `apt`/`dnf`/`pacman` on Linux.

## Advanced

Use these only when you want a narrower operation than `workstation.sh`.

- `./bootstrap/install.sh`: core toolchain, default convenience packages, latest stable Neovim, and selected optional core groups
- `./bootstrap/install-dev.sh`: baseline developer tooling, plus opt-in AI CLIs when requested
- `./bootstrap/setup-neovim.sh`: restore or update the Neovim plugin and tooling state only
- `./scripts/check`: preview stow changes
- `./scripts/stow`: apply symlinks
- `./scripts/restow`: rebuild symlinks
- `./scripts/unstow`: remove managed symlinks
