#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$script_dir/lib/common.sh"

manifests_dir=$script_dir/manifests
internal_dir=$script_dir/internal

install_recommended=1
install_antidote=1
run_check=1

while [ "$#" -gt 0 ]; do
    case $1 in
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
log "bootstrap package manager: $pm"

case $pm in
    brew)
        install_required_packages "$pm" "$manifests_dir/Brewfile.required"
        if [ "$install_recommended" -eq 1 ]; then
            install_optional_packages "$pm" "$manifests_dir/Brewfile.recommended"
        fi
        "$internal_dir/ensure-brew-shellenv.sh"
        ;;
    apt | dnf | pacman)
        install_required_packages "$pm" "$manifests_dir/packages-$pm-required.txt"
        if [ "$install_recommended" -eq 1 ]; then
            install_optional_packages "$pm" "$manifests_dir/packages-$pm-recommended.txt"
        fi
        ;;
    *)
        die "unsupported package manager: $pm"
        ;;
esac

if have_cmd git-lfs; then
    git lfs install >/dev/null 2>&1 || warn "git-lfs is installed but could not be initialized with 'git lfs install'"
fi

if [ "$install_antidote" -eq 1 ]; then
    "$internal_dir/install-antidote.sh"
fi

if [ "$run_check" -eq 1 ]; then
    "$internal_dir/check-core.sh" || true
fi
