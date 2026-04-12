# dotfiles

Personal shell and terminal configuration for `wonjae`, managed with `GNU Stow`.

This repository lives at `~/dotfiles`. That is more conventional than placing it under `~/code`, because it manages home-directory state rather than application source code.

## Layout

- `zsh/`: `zsh` startup files and plugin list
- `tmux/`: `tmux` configuration
- `git/`: shared Git config
- `starship/`: prompt configuration under `~/.config`
- `local-bin/`: small user-managed scripts under `~/.local/bin`
- `ccache/`: optional tool config under `~/.config`
- `examples/`: sample local-only files that should not be stowed
- `scripts/`: thin wrappers around `stow`

## How It Works

Each top-level package mirrors the layout under `$HOME`.

- `zsh/.zshrc` becomes `~/.zshrc`
- `tmux/.tmux.conf` becomes `~/.tmux.conf`
- `starship/.config/starship.toml` becomes `~/.config/starship.toml`

`stow` creates and removes the symlinks. The repository stores the real files.

## Tracked Files

- `~/.zshenv`
- `~/.zshrc`
- `~/.zsh_plugins.txt`
- `~/.tmux.conf`
- `~/.gitconfig`
- `~/.config/starship.toml`
- `~/.local/bin/env`
- `~/.config/ccache/ccache.conf`

## Local-Only Files

These are intentionally not tracked:

- `~/.zshrc.local`
- `~/.gitconfig.local`
- generated files such as `~/.zsh_plugins.zsh` and `~/.cache/zsh/zcompdump-*`
- installed binaries and caches under `~/.local`, `~/.cargo`, `~/.rustup`, and `~/.cache`

## First-Time Setup

On a fresh machine:

1. Install `stow`.
2. Clone this repository to `~/dotfiles`.
3. Copy `examples/gitconfig.local` to `~/.gitconfig.local` and fill in your identity.
4. Optionally copy `examples/zshrc.local` to `~/.zshrc.local` for machine-specific paths and hooks.
5. Run `./scripts/check`.
6. Run `./scripts/stow`.

On an existing machine with real files already in `$HOME`, move any conflicting files out of the way first, then run the same commands. `stow` should remain the only tool responsible for creating and removing these symlinks.

## Scripts

- `./scripts/check`: dry-run and local-file checks
- `./scripts/stow`: apply packages
- `./scripts/restow`: restow packages
- `./scripts/unstow`: remove managed symlinks

Each script also accepts package names. For example:

```sh
./scripts/check tmux zsh
./scripts/stow git starship
./scripts/restow tmux
./scripts/unstow ccache
```

The wrappers also respect `TARGET_DIR`, which is useful for testing against a temporary home directory.

```sh
TARGET_DIR=/tmp/dotfiles-test ./scripts/check
```

## Existing Machine Migration

If a target path already exists as a regular file, `stow` will refuse to overwrite it.

Typical migration flow:

1. Run `./scripts/check` and inspect conflicts.
2. Move conflicting files out of the way to a backup directory.
3. Run `./scripts/stow`.
4. Open a new shell and verify the environment.

## Notes

- `zsh` loads `~/.zshrc.local` if present.
- `git` includes `~/.gitconfig.local` for machine-local identity.
- Install external tools referenced by the configs such as `antidote`, `fzf`, `zoxide`, `starship`, and `tmux`.
