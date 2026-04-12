#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$script_dir/lib/common.sh"

manifests_dir=$script_dir/manifests
internal_dir=$script_dir/internal

run_check=1

while [ "$#" -gt 0 ]; do
    case $1 in
        --no-check)
            run_check=0
            ;;
        --package-manager=*)
            BOOTSTRAP_PACKAGE_MANAGER=${1#*=}
            export BOOTSTRAP_PACKAGE_MANAGER
            ;;
        *)
            die "unknown option: $1"
            ;;
    esac
    shift
done

install_dev_prerequisites() {
    pm=$(detect_package_manager)
    log "dev package manager: $pm"

    case $pm in
        brew)
            install_required_packages "$pm" "$manifests_dir/Brewfile.dev"
            ;;
        apt | dnf | pacman)
            install_required_packages "$pm" "$manifests_dir/packages-$pm-dev.txt"
            ;;
        *)
            die "unsupported package manager: $pm"
            ;;
    esac
}

install_uv() {
    if have_cmd uv; then
        log "uv already installed: $(command -v uv)"
        return
    fi

    have_cmd curl || die "curl is required to install uv"
    log "installing uv"
    env UV_NO_MODIFY_PATH=1 sh -c 'curl -LsSf https://astral.sh/uv/install.sh | sh'
    prepend_user_bins
    have_cmd uv || die "uv installation failed"
}

install_rustup() {
    if have_cmd rustup; then
        log "rustup already installed: $(command -v rustup)"
        return
    fi

    have_cmd curl || die "curl is required to install rustup"
    log "installing rustup"
    sh -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path"
    prepend_user_bins
    have_cmd rustup || die "rustup installation failed"
}

install_codex() {
    if have_cmd codex; then
        log "Codex CLI already installed: $(command -v codex)"
        return
    fi

    mkdir -p "$HOME/.local/bin"
    prepend_user_bins
    have_cmd npm || die "npm is required to install Codex CLI"

    log "installing Codex CLI"
    npm_config_prefix="$HOME/.local" npm i -g @openai/codex
    prepend_user_bins
    have_cmd codex || die "Codex CLI installation failed"
}

prepend_user_bins
install_dev_prerequisites
prepend_user_bins
install_uv
install_rustup
install_codex

if [ "$run_check" -eq 1 ]; then
    "$internal_dir/check-dev.sh" || true
fi
