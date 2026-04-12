#!/bin/sh
set -eu

DOTFILES_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TARGET_DIR=${TARGET_DIR:-$HOME}
DOTFILES_PACKAGES=${DOTFILES_PACKAGES:-"zsh tmux git starship local-bin ccache"}

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
