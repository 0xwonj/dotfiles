#!/bin/sh
set -eu

DOTFILES_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TARGET_DIR=${TARGET_DIR:-$HOME}
DOTFILES_PACKAGES=${DOTFILES_PACKAGES:-"shell zsh tmux git starship nvim gh btop"}
COLOR_RESET=
COLOR_BOLD=
COLOR_DIM=
COLOR_BLUE=
COLOR_GREEN=
COLOR_YELLOW=
COLOR_RED=
COLOR_CYAN=
COLOR_GRAY=

if [ -t 1 ] && [ -z "${NO_COLOR:-}" ] && [ "${TERM:-}" != "dumb" ]; then
    COLOR_RESET=$(printf '\033[0m')
    COLOR_BOLD=$(printf '\033[1m')
    COLOR_DIM=$(printf '\033[2m')
    COLOR_BLUE=$(printf '\033[34m')
    COLOR_GREEN=$(printf '\033[32m')
    COLOR_YELLOW=$(printf '\033[33m')
    COLOR_RED=$(printf '\033[31m')
    COLOR_CYAN=$(printf '\033[36m')
    COLOR_GRAY=$(printf '\033[90m')
fi

section() {
    printf '\n%s%s==>%s %s%s\n' "$COLOR_BOLD" "$COLOR_BLUE" "$COLOR_RESET" "$*" "$COLOR_RESET"
}

detail_line() {
    label=$1
    shift
    printf '  %s%-12s%s %s\n' "$COLOR_DIM" "$label" "$COLOR_RESET" "$*"
}

status_line() {
    label=$1
    shift
    label_color=$COLOR_CYAN
    display_label=$(printf '%-4s' "$label")

    case $label in
        ok)
            label_color=$COLOR_GREEN
            display_label=' ok '
            ;;
        warn | note)
            label_color=$COLOR_YELLOW
            ;;
        error | fail)
            label_color=$COLOR_RED
            ;;
        skip)
            label_color=$COLOR_GRAY
            ;;
        run)
            label_color=$COLOR_CYAN
            ;;
    esac

    printf '  [%s%s%s] %s\n' "$label_color" "$display_label" "$COLOR_RESET" "$*"
}

require_stow() {
    if ! command -v stow >/dev/null 2>&1; then
        printf '%s\n' "stow is not installed" >&2
        exit 1
    fi
}

resolve_packages() {
    packages=

    if [ "$#" -gt 0 ]; then
        for package in "$@"; do
            if [ ! -d "$DOTFILES_DIR/$package" ]; then
                printf 'unknown package: %s\n' "$package" >&2
                exit 1
            fi
            packages="${packages}${packages:+ }$package"
        done
    else
        for package in $DOTFILES_PACKAGES; do
            if [ ! -d "$DOTFILES_DIR/$package" ]; then
                printf 'missing package directory: %s\n' "$package" >&2
                exit 1
            fi
            packages="${packages}${packages:+ }$package"
        done
    fi

    printf '%s\n' "$packages"
}

has_package() {
    needle=$1
    shift

    for package in "$@"; do
        if [ "$package" = "$needle" ]; then
            return 0
        fi
    done

    return 1
}

show_usage() {
    script_name=$1
    summary=$2

    cat <<EOF
Usage: ./$script_name [package ...]

$summary

If no package names are passed, the default package set is used:
  $DOTFILES_PACKAGES

Environment:
  TARGET_DIR   Alternate stow target (default: \$HOME)
EOF
}

print_context() {
    action=$1
    packages=$2

    section "$action"
    detail_line repo "$DOTFILES_DIR"
    detail_line target "$TARGET_DIR"
    detail_line packages "$packages"
}

run_stow_action() {
    action_label=$1
    stow_flag=$2
    shift 2

    require_stow
    packages=$(resolve_packages "$@")
    print_context "$action_label" "$packages"

    cd "$DOTFILES_DIR"

    if [ -n "$stow_flag" ]; then
        # shellcheck disable=SC2086
        stow "$stow_flag" --target="$TARGET_DIR" $packages
    else
        # shellcheck disable=SC2086
        stow --target="$TARGET_DIR" $packages
    fi
}
