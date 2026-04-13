#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$script_dir/lib/common.sh"

manifests_dir=$script_dir/manifests
internal_dir=$script_dir/internal

install_recommended=1
install_antidote=1
run_check=1
upgrade_managed=0
with_github=0
with_terminal_apps=0
with_git_lfs=0

show_usage() {
    cat <<'USAGE'
Usage: ./bootstrap/install.sh [option ...]

Install the core dotfiles toolchain and baseline packages.

Options:
  --required-only                 Install only the minimum required toolchain.
  --with-github                   Install GitHub CLI.
  --with-terminal-apps            Install optional terminal applications such as tmux, btop, yazi, and starship.
  --with-git-lfs                  Install git-lfs and configure local LFS filters.
  --with-all-optional             Enable every optional core group.
  --upgrade-managed               Update baseline and selected optional packages, plus user-local tools such as starship and yazi.
                                  On Arch/pacman, installed pacman packages are not selectively upgraded.
  --skip-antidote                 Skip antidote installation or update.
  --no-check                      Skip the final core tooling check.
  --package-manager=brew|apt|dnf|pacman
                                  Override package-manager detection.
  -h, --help                      Show this help text.
USAGE
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
        --with-github)
            with_github=1
            ;;
        --with-terminal-apps)
            with_terminal_apps=1
            ;;
        --with-git-lfs)
            with_git_lfs=1
            ;;
        --with-all-optional)
            with_github=1
            with_terminal_apps=1
            with_git_lfs=1
            ;;
        --upgrade-managed | --upgrade-optional-tools)
            upgrade_managed=1
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

optional_manifest_path() {
    group=$1
    case $pm in
        brew)
            printf '%s/Brewfile.optional-%s\n' "$manifests_dir" "$group"
            ;;
        apt | dnf | pacman)
            printf '%s/packages-%s-optional-%s.txt\n' "$manifests_dir" "$pm" "$group"
            ;;
        *)
            die "unsupported package manager: $pm"
            ;;
    esac
}

install_optional_group() {
    label=$1
    group=$2
    manifest=$(optional_manifest_path "$group")
    [ -f "$manifest" ] || die "missing manifest: $manifest"
    section "$label"
    install_optional_packages "$pm" "$manifest" "$upgrade_managed"
}

install_starship() {
    if have_cmd starship && [ "$upgrade_managed" -eq 0 ]; then
        status_line skip "starship already installed: $(command -v starship)"
        return
    fi

    have_cmd curl || die "curl is required to install starship"
    have_cmd sh || die "sh is required to install starship"
    mkdir -p "$HOME/.local/bin"
    prepend_user_bins

    if [ "$upgrade_managed" -eq 1 ]; then
        status_line run "updating starship in $HOME/.local/bin"
    else
        status_line run "installing starship in $HOME/.local/bin"
    fi

    curl -fsSL https://starship.rs/install.sh | sh -s -- -y -b "$HOME/.local/bin"
    prepend_user_bins
    have_cmd starship || die "starship installation failed"
    status_line ok "starship     $(command -v starship)"
}

latest_yazi_release_json() {
    curl -fsSL -H 'Accept: application/vnd.github+json' -H 'X-GitHub-Api-Version: 2022-11-28' https://api.github.com/repos/sxyazi/yazi/releases/latest
}

extract_release_tag() {
    printf '%s\n' "$1" | awk -F'"' '/"tag_name":/ { print $4; exit }'
}

extract_asset_sha256() {
    release_json=$1
    asset_name=$2

    printf '%s\n' "$release_json" | awk -v asset="$asset_name" '
        index($0, "\"name\": \"" asset "\"") { found = 1; next }
        found && index($0, "\"digest\": \"sha256:") {
            line = $0
            sub(/.*"digest": "sha256:/, "", line)
            sub(/".*/, "", line)
            print line
            exit
        }
    '
}

sha256_file() {
    file=$1

    if have_cmd sha256sum; then
        sha256sum "$file" | awk '{ print $1 }'
        return
    fi

    if have_cmd shasum; then
        shasum -a 256 "$file" | awk '{ print $1 }'
        return
    fi

    if have_cmd openssl; then
        openssl dgst -sha256 "$file" | awk '{ print $NF }'
        return
    fi

    die "could not find a SHA-256 tool (sha256sum, shasum, or openssl)"
}

detect_yazi_asset() {
    os=$(uname -s)
    arch=$(uname -m)

    case $os in
        Darwin)
            platform=apple-darwin
            ;;
        Linux)
            platform=unknown-linux-gnu
            ;;
        *)
            die "unsupported operating system for Yazi bootstrap: $os"
            ;;
    esac

    case $arch in
        x86_64 | amd64)
            platform_arch=x86_64
            ;;
        arm64 | aarch64)
            platform_arch=aarch64
            ;;
        *)
            die "unsupported architecture for Yazi bootstrap: $arch"
            ;;
    esac

    printf 'yazi-%s-%s.zip\n' "$platform_arch" "$platform"
}

current_local_yazi_version() {
    if [ -x "$HOME/.local/bin/yazi" ]; then
        "$HOME/.local/bin/yazi" --version 2>/dev/null | awk '{ print $2; exit }'
    fi
}

install_yazi() {
    have_cmd curl || die "curl is required to install Yazi"
    have_cmd unzip || die "unzip is required to install Yazi"
    have_cmd file || die "file(1) is required by Yazi; rerun with --with-terminal-apps after installing prerequisites"

    release_json=$(latest_yazi_release_json)
    asset_name=$(detect_yazi_asset)
    latest_tag=$(extract_release_tag "$release_json")
    expected_sha=$(extract_asset_sha256 "$release_json" "$asset_name")
    current_version=$(current_local_yazi_version || true)
    latest_version=${latest_tag#v}
    version_dir="$HOME/.local/opt/yazi-$latest_version"
    stable_link="$HOME/.local/opt/yazi-stable"

    [ -n "$latest_tag" ] || die "could not resolve the latest Yazi release tag"
    [ -n "$expected_sha" ] || die "could not resolve a SHA-256 digest for $asset_name"

    if [ -n "$current_version" ] && [ "$current_version" = "$latest_version" ] && [ -x "$version_dir/yazi" ]; then
        ln -sfn "$version_dir" "$stable_link"
        ln -sfn "$version_dir/yazi" "$HOME/.local/bin/yazi"
        ln -sfn "$version_dir/ya" "$HOME/.local/bin/ya"
        status_line skip "Yazi already up to date ($current_version)"
        return
    fi

    tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/dotfiles-yazi.XXXXXX")
    archive_path="$tmp_dir/$asset_name"
    download_url="https://github.com/sxyazi/yazi/releases/download/$latest_tag/$asset_name"
    staging_dir="$tmp_dir/yazi"

    mkdir -p "$HOME/.local/opt" "$HOME/.local/bin"

    status_line run "downloading $download_url"
    curl -fL --retry 3 --retry-delay 1 --output "$archive_path" "$download_url"

    status_line run "verifying SHA-256 for $asset_name"
    actual_sha=$(sha256_file "$archive_path")
    [ "$actual_sha" = "$expected_sha" ] || die "checksum mismatch for $asset_name: expected $expected_sha, got $actual_sha"

    mkdir -p "$staging_dir"
    unzip -q "$archive_path" -d "$staging_dir"

    extracted_dir=$(find "$staging_dir" -mindepth 1 -maxdepth 1 -type d | head -n 1)
    [ -n "$extracted_dir" ] || die "downloaded Yazi archive did not contain an extracted directory"
    [ -x "$extracted_dir/yazi" ] || die "downloaded Yazi archive did not contain yazi binary"
    [ -x "$extracted_dir/ya" ] || die "downloaded Yazi archive did not contain ya binary"

    rm -rf "$version_dir"
    mv "$extracted_dir" "$version_dir"
    ln -sfn "$version_dir" "$stable_link"
    ln -sfn "$version_dir/yazi" "$HOME/.local/bin/yazi"
    ln -sfn "$version_dir/ya" "$HOME/.local/bin/ya"
    rm -rf "$tmp_dir"

    installed_version=$("$HOME/.local/bin/yazi" --version | awk '{ print $2; exit }')
    status_line ok "yazi         $HOME/.local/bin/yazi ($installed_version)"
}

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
        install_required_packages "$pm" "$manifests_dir/Brewfile.required" "$upgrade_managed"
        if [ "$install_recommended" -eq 1 ]; then
            section "default convenience packages"
            install_optional_packages "$pm" "$manifests_dir/Brewfile.recommended" "$upgrade_managed"
        fi
        section "brew shellenv"
        "$internal_dir/ensure-brew-shellenv.sh"
        ;;
    apt | dnf | pacman)
        section "required packages"
        install_required_packages "$pm" "$manifests_dir/packages-$pm-required.txt" "$upgrade_managed"
        if [ "$install_recommended" -eq 1 ]; then
            section "default convenience packages"
            install_optional_packages "$pm" "$manifests_dir/packages-$pm-recommended.txt" "$upgrade_managed"
        fi
        ;;
    *)
        die "unsupported package manager: $pm"
        ;;
esac

if [ "$with_github" -eq 1 ]; then
    install_optional_group "github tools" "github"
fi

if [ "$with_terminal_apps" -eq 1 ]; then
    install_optional_group "terminal apps" "terminal-apps"
    section "starship"
    install_starship
    section "yazi"
    install_yazi
fi

if [ "$install_antidote" -eq 1 ]; then
    section "antidote"
    "$internal_dir/install-antidote.sh"
fi

section "neovim"
"$internal_dir/install-neovim.sh"

if [ "$with_git_lfs" -eq 1 ]; then
    install_optional_group "git-lfs" "git-lfs"
    if have_cmd git-lfs; then
        section "git-lfs config"
        ensure_git_lfs_filters_in_local_config
    else
        warn "git-lfs was requested but is not available on PATH after installation"
    fi
fi

if [ "$run_check" -eq 1 ]; then
    section "core check"
    "$internal_dir/check-core.sh" || true
fi
