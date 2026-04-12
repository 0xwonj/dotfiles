#!/bin/sh
set -eu

PACKAGE_INDEX_UPDATED=0

log() {
    printf '%s\n' "$*"
}

warn() {
    printf 'warn: %s\n' "$*" >&2
}

die() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

have_cmd() {
    command -v "$1" >/dev/null 2>&1
}

prepend_path() {
    dir=$1

    [ -n "$dir" ] || return 0
    [ -d "$dir" ] || return 0

    case ":$PATH:" in
        *":$dir:"*) ;;
        *) PATH="$dir:$PATH" ;;
    esac
}

prepend_user_bins() {
    prepend_path "$HOME/.local/bin"
    prepend_path "$HOME/.cargo/bin"
    export PATH
}

run_root() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
        return
    fi

    if have_cmd sudo; then
        sudo "$@"
        return
    fi

    die "sudo is required to install packages with the detected package manager"
}

read_list_file() {
    file=$1

    while IFS= read -r line || [ -n "$line" ]; do
        case $line in
            "" | \#*)
                continue
                ;;
            *)
                printf '%s\n' "$line"
                ;;
        esac
    done < "$file"
}

detect_package_manager() {
    if [ "${BOOTSTRAP_PACKAGE_MANAGER:-}" ]; then
        printf '%s\n' "$BOOTSTRAP_PACKAGE_MANAGER"
        return
    fi

    case $(uname -s) in
        Darwin)
            if have_cmd brew; then
                printf 'brew\n'
                return
            fi
            die "Homebrew is required on macOS. Install brew first, then rerun bootstrap/install.sh."
            ;;
        Linux)
            if have_cmd apt-get; then
                printf 'apt\n'
                return
            fi
            if have_cmd dnf; then
                printf 'dnf\n'
                return
            fi
            if have_cmd pacman; then
                printf 'pacman\n'
                return
            fi
            if have_cmd brew; then
                printf 'brew\n'
                return
            fi
            ;;
    esac

    die "could not detect a supported package manager (brew, apt-get, dnf, pacman)"
}

update_package_index_if_needed() {
    pm=$1

    [ "$PACKAGE_INDEX_UPDATED" -eq 0 ] || return 0

    case $pm in
        apt)
            run_root apt-get update
            ;;
        pacman)
            run_root pacman -Sy --noconfirm
            ;;
        dnf | brew)
            ;;
        *)
            die "unsupported package manager: $pm"
            ;;
    esac

    PACKAGE_INDEX_UPDATED=1
}

install_required_packages() {
    pm=$1
    file=$2

    case $pm in
        brew)
            brew bundle --file="$file"
            return
            ;;
        apt | dnf | pacman)
            update_package_index_if_needed "$pm"
            packages=$(read_list_file "$file" | tr '\n' ' ')
            [ -n "$packages" ] || return 0
            ;;
        *)
            die "unsupported package manager: $pm"
            ;;
    esac

    case $pm in
        apt)
            # intentional word splitting for package list
            # shellcheck disable=SC2086
            run_root apt-get install -y $packages
            ;;
        dnf)
            # intentional word splitting for package list
            # shellcheck disable=SC2086
            run_root dnf install -y $packages
            ;;
        pacman)
            # intentional word splitting for package list
            # shellcheck disable=SC2086
            run_root pacman -S --needed --noconfirm $packages
            ;;
    esac
}

install_optional_packages() {
    pm=$1
    file=$2

    case $pm in
        brew)
            brew bundle --file="$file" || warn "some recommended Homebrew packages could not be installed"
            return
            ;;
        apt | dnf | pacman)
            update_package_index_if_needed "$pm"
            ;;
        *)
            die "unsupported package manager: $pm"
            ;;
    esac

    read_list_file "$file" | while IFS= read -r package; do
        case $pm in
            apt)
                run_root apt-get install -y "$package" >/dev/null 2>&1 || warn "could not install recommended apt package: $package"
                ;;
            dnf)
                run_root dnf install -y "$package" >/dev/null 2>&1 || warn "could not install recommended dnf package: $package"
                ;;
            pacman)
                run_root pacman -S --needed --noconfirm "$package" >/dev/null 2>&1 || warn "could not install recommended pacman package: $package"
                ;;
        esac
    done
}
