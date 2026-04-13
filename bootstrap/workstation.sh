#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
dotfiles_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)

show_usage() {
    cat <<'EOF'
Usage: ./bootstrap/workstation.sh [option ...]

Provision or refresh a local development workstation end-to-end:
  1. install core tooling
  2. install or update development tooling
  3. stow the selected dotfiles packages
  4. provision or update the Neovim environment

Options:
  --update                        Refresh repo-managed packages, developer tooling, and Neovim state.
                                  On Arch/pacman, installed pacman packages are not selectively upgraded.
                                  Does not perform a full system package upgrade.
  --with-github                   Include GitHub CLI.
  --with-terminal-apps            Include optional terminal applications, starship, and related dotfiles.
  --with-git-lfs                  Include git-lfs and configure local LFS filters.
  --with-ai-tools                 Include account-scoped AI CLIs such as codex and claude.
  --with-all-optional             Enable every optional group.
  --package-manager=brew|apt|dnf|pacman
                                  Override package-manager detection.
  --no-check                      Skip post-install checks where supported.
  -h, --help                      Show this help text.
EOF
}

pm_arg=
no_check=0
update_mode=0
with_github=0
with_terminal_apps=0
with_git_lfs=0
with_ai_tools=0

while [ "$#" -gt 0 ]; do
    case $1 in
        -h | --help)
            show_usage
            exit 0
            ;;
        --update)
            update_mode=1
            ;;
        --with-github)
            with_github=1
            ;;
        --with-terminal-apps)
            with_terminal_apps=1
            ;;
        --with-git-lfs)
            with_git_lfs=1
            ;;
        --with-ai-tools)
            with_ai_tools=1
            ;;
        --with-all-optional)
            with_github=1
            with_terminal_apps=1
            with_git_lfs=1
            with_ai_tools=1
            ;;
        --package-manager=*)
            pm_arg=$1
            ;;
        --no-check)
            no_check=1
            ;;
        *)
            printf 'error: unknown option: %s\n' "$1" >&2
            exit 1
            ;;
    esac
    shift
done

run_step() {
    title=$1
    shift

    printf '\n==> %s\n' "$title"
    "$@"
}

run_install_step() {
    set -- "$script_dir/install.sh"
    [ -n "$pm_arg" ] && set -- "$@" "$pm_arg"
    [ "$no_check" -eq 1 ] && set -- "$@" "--no-check"
    [ "$update_mode" -eq 1 ] && set -- "$@" "--upgrade-managed"
    [ "$with_github" -eq 1 ] && set -- "$@" "--with-github"
    [ "$with_terminal_apps" -eq 1 ] && set -- "$@" "--with-terminal-apps"
    [ "$with_git_lfs" -eq 1 ] && set -- "$@" "--with-git-lfs"
    run_step "core bootstrap" "$@"
}

run_dev_step() {
    set -- "$script_dir/install-dev.sh"
    [ "$update_mode" -eq 1 ] && set -- "$@" "--upgrade-tools"
    [ -n "$pm_arg" ] && set -- "$@" "$pm_arg"
    [ "$no_check" -eq 1 ] && set -- "$@" "--no-check"
    [ "$with_ai_tools" -eq 1 ] && set -- "$@" "--with-ai-tools"
    if [ "$update_mode" -eq 1 ]; then
        run_step "developer tooling update" "$@"
    else
        run_step "developer bootstrap" "$@"
    fi
}

run_stow_step() {
    set -- shell zsh git nvim
    [ "$with_terminal_apps" -eq 1 ] && set -- "$@" tmux btop starship

    run_step "check" "$dotfiles_dir/scripts/check" "$@"
    run_step "stow" "$dotfiles_dir/scripts/stow" "$@"
}

run_setup_step() {
    set -- "$script_dir/setup-neovim.sh"
    [ "$update_mode" -eq 1 ] && set -- "$@" "--update"
    [ "$no_check" -eq 1 ] && set -- "$@" "--no-check"
    if [ "$update_mode" -eq 1 ]; then
        run_step "neovim update" "$@"
    else
        run_step "neovim setup" "$@"
    fi
}

run_install_step
run_dev_step
run_stow_step
run_setup_step
