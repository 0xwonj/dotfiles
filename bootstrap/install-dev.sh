#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$script_dir/lib/common.sh"

manifests_dir=$script_dir/manifests
internal_dir=$script_dir/internal

run_check=1
upgrade_tools=0
with_ai_tools=0

show_usage() {
    cat <<'EOF'
Usage: ./bootstrap/install-dev.sh [option ...]

Install developer tooling that should live outside the stowed dotfiles.

Options:
  --with-ai-tools                 Install opt-in AI CLIs managed outside the shared baseline.
  --with-all-optional             Enable every optional developer group.
  --upgrade-tools                 Update managed developer tooling, including package-manager packages plus uv, rustup, and selected AI CLIs.
                                  On Arch/pacman, installed pacman packages are not selectively upgraded.
  --no-check                      Skip the final development tooling check.
  --package-manager=brew|apt|dnf|pacman
                                  Override package-manager detection.
  -h, --help                      Show this help text.
EOF
}

while [ "$#" -gt 0 ]; do
    case $1 in
        -h | --help)
            show_usage
            exit 0
            ;;
        --no-check)
            run_check=0
            ;;
        --upgrade-tools)
            upgrade_tools=1
            ;;
        --with-ai-tools)
            with_ai_tools=1
            ;;
        --with-all-optional)
            with_ai_tools=1
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
    detail_line "package mgr" "$pm"

    case $pm in
        apt | dnf | pacman)
            ensure_root_access
            start_sudo_keepalive
            trap 'stop_sudo_keepalive' EXIT HUP INT TERM
            ;;
    esac

    case $pm in
        brew)
            install_required_packages "$pm" "$manifests_dir/Brewfile.dev" "$upgrade_tools"
            ;;
        apt | dnf | pacman)
            install_required_packages "$pm" "$manifests_dir/packages-$pm-dev.txt" "$upgrade_tools"
            ;;
        *)
            die "unsupported package manager: $pm"
            ;;
    esac
}

install_uv() {
    if have_cmd uv; then
        if [ "$upgrade_tools" -eq 1 ]; then
            log "updating uv"
            uv self update
        else
            log "uv already installed: $(command -v uv)"
        fi
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
        if [ "$upgrade_tools" -eq 1 ]; then
            log "updating rustup"
            rustup self update
            rustup update
        else
            log "rustup already installed: $(command -v rustup)"
        fi
        return
    fi

    have_cmd curl || die "curl is required to install rustup"
    log "installing rustup"
    sh -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path"
    prepend_user_bins
    have_cmd rustup || die "rustup installation failed"
}

install_codex() {
    if have_cmd codex && [ "$upgrade_tools" -eq 0 ]; then
        log "Codex CLI already installed: $(command -v codex)"
        return
    fi

    mkdir -p "$HOME/.local/bin"
    prepend_user_bins
    have_cmd npm || die "npm is required to install Codex CLI"

    if [ "$upgrade_tools" -eq 1 ]; then
        log "updating Codex CLI"
        npm_config_prefix="$HOME/.local" npm i -g @openai/codex@latest
    else
        log "installing Codex CLI"
        npm_config_prefix="$HOME/.local" npm i -g @openai/codex
    fi
    prepend_user_bins
    have_cmd codex || die "Codex CLI installation failed"
}

install_claude() {
    channel=${CLAUDE_CODE_CHANNEL:-stable}

    if have_cmd claude && [ "$upgrade_tools" -eq 0 ]; then
        log "Claude Code already installed: $(command -v claude)"
        return
    fi

    have_cmd curl || die "curl is required to install Claude Code"
    have_cmd bash || die "bash is required to install Claude Code"
    mkdir -p "$HOME/.local/bin"
    prepend_user_bins

    if [ "$upgrade_tools" -eq 1 ]; then
        log "updating Claude Code ($channel)"
    else
        log "installing Claude Code ($channel)"
    fi

    env PATH="$PATH" CLAUDE_CODE_CHANNEL="$channel" sh -c 'curl -fsSL https://claude.ai/install.sh | bash -s -- "$CLAUDE_CODE_CHANNEL"'
    prepend_user_bins
    have_cmd claude || die "Claude Code installation failed"
}

prepend_user_bins
section "development prerequisites"
detail_line mode "developer"
install_dev_prerequisites
prepend_user_bins
section "uv"
install_uv
section "rustup"
install_rustup

if [ "$with_ai_tools" -eq 1 ]; then
    section "agent tools"
    install_codex
    install_claude
fi

if [ "$run_check" -eq 1 ]; then
    section "dev check"
    "$internal_dir/check-dev.sh" || true
fi
