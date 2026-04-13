#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$script_dir/lib/common.sh"

prepend_user_bins
prepend_brew_path_if_present

latest_release_json() {
    curl -fsSL -H 'Accept: application/vnd.github+json' -H 'X-GitHub-Api-Version: 2022-11-28' https://api.github.com/repos/neovim/neovim/releases/latest
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

detect_neovim_asset() {
    os=$(uname -s)
    arch=$(uname -m)

    case $os in
        Darwin)
            platform=macos
            ;;
        Linux)
            platform=linux
            ;;
        *)
            die "unsupported operating system for Neovim bootstrap: $os"
            ;;
    esac

    case $arch in
        x86_64 | amd64)
            platform_arch=x86_64
            ;;
        arm64 | aarch64)
            platform_arch=arm64
            ;;
        *)
            die "unsupported architecture for Neovim bootstrap: $arch"
            ;;
    esac

    printf 'nvim-%s-%s.tar.gz\n' "$platform" "$platform_arch"
}

current_local_neovim_version() {
    if [ -x "$HOME/.local/bin/nvim" ]; then
        "$HOME/.local/bin/nvim" --version 2>/dev/null | awk 'NR==1 { print $2 }'
    fi
}

tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/dotfiles-nvim.XXXXXX")
trap 'rm -rf "$tmp_dir"' EXIT HUP INT TERM

release_json=$(latest_release_json)
latest_tag=$(extract_release_tag "$release_json")
asset_name=$(detect_neovim_asset)
expected_sha=$(extract_asset_sha256 "$release_json" "$asset_name")
current_tag=$(current_local_neovim_version || true)
version_dir="$HOME/.local/opt/nvim-$latest_tag"
stable_link="$HOME/.local/opt/nvim-stable"
bin_link="$HOME/.local/bin/nvim"

[ -n "$latest_tag" ] || die "could not resolve the latest Neovim release tag"
[ -n "$expected_sha" ] || die "could not resolve a SHA-256 digest for $asset_name"

detail_line release "$latest_tag"
detail_line asset "$asset_name"
detail_line install "$version_dir"

mkdir -p "$HOME/.local/opt" "$HOME/.local/bin"

if [ -n "$current_tag" ] && [ "$current_tag" = "$latest_tag" ] && [ -x "$version_dir/bin/nvim" ]; then
    ln -sfn "$version_dir" "$stable_link"
    ln -sfn "$version_dir/bin/nvim" "$bin_link"
    status_line skip "Neovim already up to date ($current_tag)"
    exit 0
fi

archive_path="$tmp_dir/$asset_name"
download_url="https://github.com/neovim/neovim/releases/download/$latest_tag/$asset_name"
staging_dir="$tmp_dir/nvim"

status_line run "downloading $download_url"
curl -fL --retry 3 --retry-delay 1 --output "$archive_path" "$download_url"

status_line run "verifying SHA-256 for $asset_name"
actual_sha=$(sha256_file "$archive_path")
[ "$actual_sha" = "$expected_sha" ] || die "checksum mismatch for $asset_name: expected $expected_sha, got $actual_sha"

mkdir -p "$staging_dir"
tar -xzf "$archive_path" -C "$staging_dir" --strip-components=1

[ -x "$staging_dir/bin/nvim" ] || die "downloaded Neovim archive did not contain bin/nvim"

rm -rf "$version_dir"
mv "$staging_dir" "$version_dir"
ln -sfn "$version_dir" "$stable_link"
ln -sfn "$version_dir/bin/nvim" "$bin_link"

installed_version=$("$bin_link" --version | awk 'NR==1 { print $2 }')
status_line ok "Neovim installed ($installed_version)"
detail_line binary "$bin_link"
