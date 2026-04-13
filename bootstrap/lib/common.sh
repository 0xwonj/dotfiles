#!/bin/sh
set -eu

PACKAGE_INDEX_UPDATED=0
SUDO_KEEPALIVE_PID=
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

log() {
    printf '%s\n' "$*"
}

section() {
    printf '\n%s%s==>%s %s%s\n' "$COLOR_BOLD" "$COLOR_BLUE" "$COLOR_RESET" "$*" "$COLOR_RESET"
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

detail_line() {
    label=$1
    shift
    printf '  %s%-12s%s %s\n' "$COLOR_DIM" "$label" "$COLOR_RESET" "$*"
}

warn() {
    printf '%swarn:%s %s\n' "$COLOR_YELLOW" "$COLOR_RESET" "$*" >&2
}

die() {
    printf '%serror:%s %s\n' "$COLOR_RED" "$COLOR_RESET" "$*" >&2
    exit 1
}

have_cmd() {
    command -v "$1" >/dev/null 2>&1
}

command_for_package() {
    case $1 in
        neovim)
            printf 'nvim\n'
            ;;
        github-cli)
            printf 'gh\n'
            ;;
        nodejs)
            printf 'node\n'
            ;;
        *)
            printf '%s\n' "$1"
            ;;
    esac
}

package_command_available() {
    package=$1
    command_name=$(command_for_package "$package")
    have_cmd "$command_name"
}

package_installed() {
    pm=$1
    package=$2

    case $pm in
        apt)
            dpkg-query -W -f='${Status}' "$package" 2>/dev/null | grep -q '^install ok installed$'
            ;;
        dnf)
            rpm -q "$package" >/dev/null 2>&1
            ;;
        pacman)
            pacman -Q "$package" >/dev/null 2>&1
            ;;
        brew)
            brew_bin=$(resolve_brew 2>/dev/null || true)
            [ -n "$brew_bin" ] || return 1
            "$brew_bin" list --formula "$package" >/dev/null 2>&1
            ;;
        *)
            return 1
            ;;
    esac
}

package_available() {
    pm=$1
    package=$2

    if package_command_available "$package"; then
        return 0
    fi

    package_installed "$pm" "$package"
}

ensure_local_gitconfig_file() {
    target_file=${1:-$HOME/.gitconfig.local}

    if [ -L "$target_file" ]; then
        die "$target_file exists as a symlink; use a regular local file for machine-specific git overrides"
    fi

    if [ -e "$target_file" ] && [ ! -f "$target_file" ]; then
        die "$target_file exists but is not a regular file"
    fi

    if [ ! -f "$target_file" ]; then
        cat <<'EOF' > "$target_file"
# Machine-specific Git configuration belongs here.
# Use this file for identity, signing keys, and machine-local integrations.
EOF
    fi
}

ensure_git_lfs_filters_in_local_config() {
    target_file=${1:-$HOME/.gitconfig.local}

    ensure_local_gitconfig_file "$target_file"

    git config -f "$target_file" filter.lfs.clean "git-lfs clean -- %f"
    git config -f "$target_file" filter.lfs.smudge "git-lfs smudge -- %f"
    git config -f "$target_file" filter.lfs.process "git-lfs filter-process"
    git config -f "$target_file" filter.lfs.required true

    status_line ok "git-lfs filters in $target_file"
}

resolve_brew() {
    if [ "${HOMEBREW_BREW_FILE:-}" ] && [ -x "${HOMEBREW_BREW_FILE:-}" ]; then
        printf '%s\n' "$HOMEBREW_BREW_FILE"
        return 0
    fi

    for brew_bin in \
        /opt/homebrew/bin/brew \
        /usr/local/bin/brew \
        /home/linuxbrew/.linuxbrew/bin/brew
    do
        if [ -x "$brew_bin" ]; then
            printf '%s\n' "$brew_bin"
            return 0
        fi
    done

    if have_cmd brew; then
        command -v brew
        return 0
    fi

    return 1
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

prepend_brew_path_if_present() {
    brew_bin=$(resolve_brew 2>/dev/null || true)
    [ -n "$brew_bin" ] || return 0
    prepend_path "${brew_bin%/brew}"
    export PATH
}

run_root() {
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
        return
    fi

    if have_cmd sudo; then
        if sudo -n true >/dev/null 2>&1; then
            sudo -n "$@"
            return
        fi

        die "sudo privileges are required for '$*'; run 'sudo -v' first or rerun the bootstrap script interactively"
    fi

    die "sudo is required to install packages with the detected package manager"
}

ensure_root_access() {
    if [ "$(id -u)" -eq 0 ]; then
        return
    fi

    have_cmd sudo || die "sudo is required to install packages with the detected package manager"
    section "sudo authentication"
    sudo -v
}

start_sudo_keepalive() {
    if [ "$(id -u)" -eq 0 ]; then
        return
    fi

    have_cmd sudo || return

    (
        while :; do
            sudo -n true >/dev/null 2>&1 || exit
            sleep 30
        done
    ) &
    SUDO_KEEPALIVE_PID=$!
}

stop_sudo_keepalive() {
    if [ -n "${SUDO_KEEPALIVE_PID:-}" ]; then
        kill "$SUDO_KEEPALIVE_PID" >/dev/null 2>&1 || true
        wait "$SUDO_KEEPALIVE_PID" 2>/dev/null || true
        SUDO_KEEPALIVE_PID=
    fi
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

count_list_file() {
    file=$1
    count=0

    while IFS= read -r _line || [ -n "$_line" ]; do
        case $_line in
            "" | \#*)
                continue
                ;;
            *)
                count=$((count + 1))
                ;;
        esac
    done < "$file"

    printf '%s\n' "$count"
}

install_package_with_pm() {
    pm=$1
    package=$2

    case $pm in
        apt)
            run_root env DEBIAN_FRONTEND=noninteractive apt-get install -y "$package" >/dev/null 2>&1
            ;;
        dnf)
            run_root dnf install -y "$package" >/dev/null 2>&1
            ;;
        pacman)
            run_root pacman -S --needed --noconfirm "$package" >/dev/null 2>&1
            ;;
        brew)
            brew_bin=$(resolve_brew) || die "Homebrew is required but brew could not be found"
            "$brew_bin" install "$package" >/dev/null 2>&1
            ;;
        *)
            die "unsupported package manager: $pm"
            ;;
    esac
}

detect_package_manager() {
    if [ "${BOOTSTRAP_PACKAGE_MANAGER:-}" ]; then
        printf '%s\n' "$BOOTSTRAP_PACKAGE_MANAGER"
        return
    fi

    case $(uname -s) in
        Darwin)
            if resolve_brew >/dev/null 2>&1; then
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
            if resolve_brew >/dev/null 2>&1; then
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
        dnf | brew | pacman)
            ;;
        *)
            die "unsupported package manager: $pm"
            ;;
    esac

    PACKAGE_INDEX_UPDATED=1
}

upgrade_package_with_pm() {
    pm=$1
    package=$2

    case $pm in
        apt)
            if package_installed "$pm" "$package"; then
                run_root env DEBIAN_FRONTEND=noninteractive apt-get install --only-upgrade -y "$package" >/dev/null 2>&1
            else
                run_root env DEBIAN_FRONTEND=noninteractive apt-get install -y "$package" >/dev/null 2>&1
            fi
            ;;
        dnf)
            if package_installed "$pm" "$package"; then
                run_root dnf upgrade -y "$package" >/dev/null 2>&1
            else
                run_root dnf install -y "$package" >/dev/null 2>&1
            fi
            ;;
        pacman)
            run_root pacman -S --needed --noconfirm "$package" >/dev/null 2>&1
            ;;
        brew)
            brew_bin=$(resolve_brew) || die "Homebrew is required but brew could not be found"
            if package_installed "$pm" "$package"; then
                "$brew_bin" upgrade "$package" >/dev/null 2>&1
            else
                "$brew_bin" install "$package" >/dev/null 2>&1
            fi
            ;;
        *)
            die "unsupported package manager: $pm"
            ;;
    esac
}

sync_manifest_packages() {
    pm=$1
    file=$2
    required=$3
    upgrade=$4
    manifest_name=$(basename "$file")
    package_count=$(count_list_file "$file")

    detail_line manifest "$manifest_name ($package_count entries)"

    if [ "$upgrade" -eq 1 ] && [ "$pm" = "pacman" ]; then
        status_line note "pacman package upgrades are skipped; run 'sudo pacman -Syu' separately"
    fi

    case $pm in
        apt | dnf | pacman)
            update_package_index_if_needed "$pm"
            ;;
        brew)
            :
            ;;
        *)
            die "unsupported package manager: $pm"
            ;;
    esac

    current=0
    while IFS= read -r package || [ -n "$package" ]; do
        case $package in
            "" | \#*)
                continue
                ;;
        esac

        current=$((current + 1))

        if [ "$upgrade" -eq 1 ]; then
            if [ "$pm" = "pacman" ] && package_installed "$pm" "$package"; then
                status_line skip "[$current/$package_count] $package (managed by pacman; no selective upgrade)"
                continue
            fi

            if package_installed "$pm" "$package"; then
                status_line run "[$current/$package_count] updating $package"
                if upgrade_package_with_pm "$pm" "$package"; then
                    status_line ok "[$current/$package_count] $package"
                elif [ "$required" -eq 1 ]; then
                    die "could not update required $pm package: $package"
                else
                    warn "[$current/$package_count] could not update optional $pm package: $package"
                fi
                continue
            fi

            if package_command_available "$package"; then
                status_line skip "[$current/$package_count] $package (provided externally)"
                continue
            fi

            status_line run "[$current/$package_count] installing $package"
            if install_package_with_pm "$pm" "$package"; then
                status_line ok "[$current/$package_count] $package"
            elif [ "$required" -eq 1 ]; then
                die "could not install required $pm package: $package"
            else
                warn "[$current/$package_count] could not install optional $pm package: $package"
            fi
            continue
        fi

        if package_available "$pm" "$package"; then
            status_line skip "[$current/$package_count] $package"
            continue
        fi

        status_line run "[$current/$package_count] installing $package"
        if install_package_with_pm "$pm" "$package"; then
            status_line ok "[$current/$package_count] $package"
        elif [ "$required" -eq 1 ]; then
            die "could not install required $pm package: $package"
        else
            warn "[$current/$package_count] could not install optional $pm package: $package"
        fi
    done < "$file"
}

install_required_packages() {
    pm=$1
    file=$2
    upgrade=${3:-0}
    sync_manifest_packages "$pm" "$file" 1 "$upgrade"
}

install_optional_packages() {
    pm=$1
    file=$2
    upgrade=${3:-0}
    sync_manifest_packages "$pm" "$file" 0 "$upgrade"
}
