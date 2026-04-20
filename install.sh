#!/bin/sh
set -eu

repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

show_usage() {
  cat <<'USAGE'
Usage: ./install.sh [dotctl bootstrap options...]

Install Rust if needed, install dotctl into ~/.local/bin, then hand off to
interactive dotctl bootstrap.

Examples:
  ./install.sh
  ./install.sh --profile laptop
  ./install.sh --no-prompt --git-name "Your Name" --git-email you@example.com
USAGE
}

case "${1:-}" in
  -h|--help)
    show_usage
    exit 0
    ;;
esac

have_cmd() {
  command -v "$1" >/dev/null 2>&1
}

have_dotctl_build_prereqs() {
  have_cmd cargo && have_cmd rustc && have_cmd curl
}

ensure_sudo() {
  if [ "$(id -u)" -eq 0 ]; then
    return 0
  fi
  if ! have_cmd sudo; then
    printf 'error: sudo is required to install dotctl prerequisites on this machine\n' >&2
    exit 1
  fi
  sudo -v
}

install_linux_build_prereqs() {
  if have_cmd apt-get; then
    ensure_sudo
    sudo apt-get update
    sudo apt-get install -y build-essential pkg-config libssl-dev curl git
  elif have_cmd dnf; then
    ensure_sudo
    sudo dnf install -y gcc gcc-c++ make pkgconf-pkg-config openssl-devel curl git
  elif have_cmd pacman; then
    ensure_sudo
    sudo pacman -Syu --needed --noconfirm base-devel openssl curl git
  else
    printf 'error: unsupported Linux package manager for initial dotctl installation\n' >&2
    exit 1
  fi
}

ensure_dotctl_build_prereqs() {
  if have_dotctl_build_prereqs; then
    return 0
  fi

  case $(uname -s) in
    Darwin)
      if ! xcode-select -p >/dev/null 2>&1; then
        printf 'error: Xcode Command Line Tools are required. Run xcode-select --install first.\n' >&2
        exit 1
      fi
      if ! have_cmd curl; then
        printf 'error: curl is required to install rustup on macOS.\n' >&2
        exit 1
      fi
      ;;
    Linux)
      install_linux_build_prereqs
      ;;
    *)
      printf 'error: unsupported operating system\n' >&2
      exit 1
      ;;
  esac
}

install_rustup() {
  if have_cmd cargo && have_cmd rustc; then
    return 0
  fi
  if ! have_cmd curl; then
    printf 'error: curl is required to install rustup\n' >&2
    exit 1
  fi
  tmp_script=$(mktemp "${TMPDIR:-/tmp}/dotctl-rustup.XXXXXX")
  trap 'rm -f "$tmp_script"' EXIT HUP INT TERM
  curl -fsSL https://sh.rustup.rs -o "$tmp_script"
  sh "$tmp_script" -y --no-modify-path
  rm -f "$tmp_script"
  trap - EXIT HUP INT TERM
}

install_dotctl() {
  export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
  mkdir -p "$HOME/.local"
  cargo install --locked --path "$repo_dir/crates/dotctl" --root "$HOME/.local"
}

ensure_dotctl_build_prereqs
install_rustup
install_dotctl

export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
exec "$HOME/.local/bin/dotctl" bootstrap --repo "$repo_dir" "$@"
