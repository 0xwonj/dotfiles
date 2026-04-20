# dotfiles

Personal workstation configuration for `wonjae`, managed by `chezmoi` and orchestrated by `dotctl`.

## Quick Start

Fresh machine:

```sh
./install.sh
```

This installs Rust if needed, installs `dotctl`, and then hands off to interactive `dotctl bootstrap`. If a fresh Linux machine is missing the minimum build prerequisites for `dotctl`, `install.sh` installs only those prerequisites first.

Daily use:

```sh
dotctl update
```

Useful narrow commands:

```sh
dotctl diff
dotctl apply
dotctl doctor
dotctl state show
dotctl state edit
dotctl features list
```

## Layout

- `home/`: `chezmoi` source-state applied into `$HOME`
- `config/profiles/`: shared profile defaults
- `config/bundles/`: feature-to-package/tool bundle mapping
- `config/installers.toml`: single source of truth for managed tool installer metadata
- `crates/dotctl`: user-facing Rust CLI
- `crates/dotctl-core`: state resolution and orchestration engine
- `docs/target-architecture.md`: target architecture and rationale

## State

- `~/.config/dotfiles/local.toml`: authoritative machine-local state
- `~/.config/dotfiles/state.toml`: generated snapshot written only after a successful `bootstrap` or `update` run
- `~/.config/chezmoi/chezmoi.toml`: generated `chezmoi` config
- `~/.gitconfig.extra`, `~/.zprofile.local`, `~/.zshrc.local`: unmanaged local extension points

## Features

Available feature flags:

- `github`
- `terminal_apps`
- `git_lfs`
- `ai_tools`
- `fastfetch`

Feature defaults come from the selected profile and are persisted in `local.toml`.

## Notes

- `dotctl bootstrap` is the end-to-end converge path for a new machine.
- `dotctl update` reuses stored machine-local state and refreshes packages, user-local tools, managed files, and post-apply sync targets.
- `dotctl apply` only refreshes generated `chezmoi` config, applies `home/`, and runs minimal post-apply tasks.
- `dotctl doctor` fails on required-tool or runtime-health regressions.
- `chezmoi` remains the file engine; `dotctl` owns state, policy, installers, and health checks.
