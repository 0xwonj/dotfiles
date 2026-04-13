#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$script_dir/lib/common.sh"

run_check=1
update_mode=0

show_usage() {
    cat <<'EOF'
Usage: ./bootstrap/setup-neovim.sh [option ...]

Install Neovim-adjacent tooling and provision the stowed Neovim config.

Options:
  --update                        Intentionally update Neovim plugins and managed tools.
  --no-check                      Skip the final Neovim smoke check.
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
        --update)
            update_mode=1
            ;;
        *)
            die "unknown option: $1"
            ;;
    esac
    shift
done

require_cmd() {
    command_name=$1
    hint=${2:-}

    if have_cmd "$command_name"; then
        status_line ok "$(printf '%-12s %s' "$command_name" "$(command -v "$command_name")")"
        return
    fi

    if [ -n "$hint" ]; then
        die "missing required command: $command_name ($hint)"
    fi

    die "missing required command: $command_name"
}

run_nvim_headless() {
    log_file=$(mktemp "${TMPDIR:-/tmp}/dotfiles-nvim.XXXXXX")
    if nvim --headless "$@" >"$log_file" 2>&1; then
        rm -f "$log_file"
        return 0
    fi

    cat "$log_file" >&2
    rm -f "$log_file"
    return 1
}

have_uv_tool() {
    tool_name=$1
    uv tool list 2>/dev/null | awk '{ print $1 }' | grep -qx "$tool_name"
}

have_cargo_install() {
    package_name=$1
    cargo install --list 2>/dev/null | grep -Eq "^$package_name v"
}

prepend_user_bins
prepend_brew_path_if_present

section "neovim setup"
detail_line config "$HOME/.config/nvim"
if [ "$update_mode" -eq 1 ]; then
    detail_line mode "update"
else
    detail_line mode "bootstrap"
fi

[ -f "$HOME/.config/nvim/init.lua" ] || die "Neovim config is not stowed. Run ./scripts/stow nvim first."

if [ "$(uname -s)" = "Darwin" ] && ! xcode-select -p >/dev/null 2>&1; then
    die "Xcode Command Line Tools are required on macOS. Run 'xcode-select --install' and rerun this script."
fi

printf '\n'
require_cmd nvim "run ./bootstrap/install.sh first"
require_cmd uv "run ./bootstrap/install-dev.sh first"
require_cmd cargo "run ./bootstrap/install-dev.sh first"
require_cmd node "run ./bootstrap/install-dev.sh first"
require_cmd npm "run ./bootstrap/install-dev.sh first"
require_cmd git
require_cmd curl
require_cmd tar
require_cmd unzip "install.sh should provide this"
require_cmd make "install-dev.sh should provide compiler tools"
require_cmd cc "install-dev.sh should provide compiler tools"

section "python provider"
if [ "$update_mode" -eq 1 ]; then
    status_line run "updating pynvim with uv"
    uv tool install --upgrade pynvim
elif have_uv_tool pynvim; then
    status_line skip "pynvim already installed"
else
    status_line run "installing pynvim with uv"
    uv tool install pynvim
fi

section "tree-sitter cli"
if [ "$update_mode" -eq 1 ]; then
    status_line run "updating tree-sitter-cli with cargo"
    cargo install tree-sitter-cli --locked --force
elif have_cargo_install tree-sitter-cli; then
    status_line skip "tree-sitter-cli already installed"
else
    status_line run "installing tree-sitter-cli with cargo"
    cargo install tree-sitter-cli --locked
fi

section "plugins"
if [ "$update_mode" -eq 1 ]; then
    status_line run "updating lazy.nvim plugins and lockfile"
    run_nvim_headless '+Lazy! update' '+Lazy! clean' '+qall'
else
    status_line run "restoring lazy.nvim plugins from lazy-lock.json"
    run_nvim_headless '+Lazy! restore' '+Lazy! clean' '+qall'
fi

section "tree-sitter parsers"
treesitter_languages=$(nvim --headless '+lua require("config.bootstrap_tasks").print_treesitter_languages()' '+qall')
if [ -n "$treesitter_languages" ]; then
    status_line run "ensuring: $treesitter_languages"
    run_nvim_headless '+lua require("config.bootstrap_tasks").ensure_treesitter_parsers()' '+qall'
else
    status_line skip "no tree-sitter parsers configured"
fi

section "mason packages"
mason_packages=$(nvim --headless '+lua require("config.bootstrap_tasks").print_mason_packages()' '+qall')
missing_mason_packages=$(nvim --headless '+lua require("config.bootstrap_tasks").print_missing_mason_packages()' '+qall')
if [ -z "$mason_packages" ]; then
    status_line skip "no Mason packages configured"
elif [ "$update_mode" -eq 1 ]; then
    status_line run "updating Mason registry"
    run_nvim_headless -c 'MasonUpdate' -c 'qall'
    status_line run "refreshing: $mason_packages"
    run_nvim_headless '+lua require("config.bootstrap_tasks").ensure_mason_packages({ update = true })' '+qall'
elif [ -n "$missing_mason_packages" ]; then
    status_line run "installing missing: $missing_mason_packages"
    run_nvim_headless '+lua require("config.bootstrap_tasks").ensure_mason_packages()' '+qall'
else
    status_line skip "Mason packages already installed"
fi

if [ "$run_check" -eq 1 ]; then
    section "smoke check"
    status_line run "loading Neovim config"
    run_nvim_headless '+qall'
    status_line ok "Neovim config loaded successfully"
fi
