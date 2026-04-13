#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$script_dir/lib/common.sh"

manifests_dir=$script_dir/manifests
internal_dir=$script_dir/internal

install_recommended=1
install_antidote=1
run_check=1

show_usage() {
    cat <<'EOF'
Usage: ./bootstrap/install.sh [option ...]

Install the core dotfiles toolchain and recommended packages.

Options:
  --required-only                 Install only the minimum required toolchain.
  --skip-antidote                 Skip antidote installation or update.
  --no-check                      Skip the final core tooling check.
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
        --required-only)
            install_recommended=0
            install_antidote=0
            ;;
        --skip-antidote)
            install_antidote=0
            ;;
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

pm=$(detect_package_manager)
section "bootstrap ($pm)"
detail_line mode "core"

case $pm in
    apt | dnf | pacman)
        ensure_root_access
        start_sudo_keepalive
        trap 'stop_sudo_keepalive' EXIT HUP INT TERM
        ;;
esac

case $pm in
    brew)
        section "required packages"
        install_required_packages "$pm" "$manifests_dir/Brewfile.required"
        if [ "$install_recommended" -eq 1 ]; then
            section "recommended packages"
            install_optional_packages "$pm" "$manifests_dir/Brewfile.recommended"
        fi
        section "brew shellenv"
        "$internal_dir/ensure-brew-shellenv.sh"
        ;;
    apt | dnf | pacman)
        section "required packages"
        install_required_packages "$pm" "$manifests_dir/packages-$pm-required.txt"
        if [ "$install_recommended" -eq 1 ]; then
            section "recommended packages"
            install_optional_packages "$pm" "$manifests_dir/packages-$pm-recommended.txt"
        fi
        ;;
    *)
        die "unsupported package manager: $pm"
        ;;
esac

if have_cmd git-lfs; then
    section "git-lfs"
    ensure_git_lfs_filters_in_local_config
fi

if [ "$install_antidote" -eq 1 ]; then
    section "antidote"
    "$internal_dir/install-antidote.sh"
fi

if [ "$run_check" -eq 1 ]; then
    section "core check"
    "$internal_dir/check-core.sh" || true
fi
