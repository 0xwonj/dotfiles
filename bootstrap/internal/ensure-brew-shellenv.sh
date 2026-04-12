#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$script_dir/lib/common.sh"

target_home=${TARGET_HOME:-$HOME}
target_file=$target_home/.zprofile.local
start_marker="# >>> dotfiles brew shellenv >>>"
end_marker="# <<< dotfiles brew shellenv <<<"

managed_block() {
    cat <<'EOF'
# >>> dotfiles brew shellenv >>>
if [ -x /opt/homebrew/bin/brew ]; then
  eval "$(/opt/homebrew/bin/brew shellenv)"
elif [ -x /usr/local/bin/brew ]; then
  eval "$(/usr/local/bin/brew shellenv)"
elif [ -x /home/linuxbrew/.linuxbrew/bin/brew ]; then
  eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"
elif command -v brew >/dev/null 2>&1; then
  eval "$(brew shellenv)"
fi
# <<< dotfiles brew shellenv <<<
EOF
}

if [ -L "$target_file" ]; then
    die "$target_file exists as a symlink; use a regular local file for machine-specific zsh overrides"
fi

if [ -e "$target_file" ] && [ ! -f "$target_file" ]; then
    die "$target_file exists but is not a regular file"
fi

mkdir -p "$target_home"
tmp_file=$(mktemp "${TMPDIR:-/tmp}/dotfiles-zprofile-local.XXXXXX")
trap 'rm -f "$tmp_file"' EXIT HUP INT TERM

if [ -f "$target_file" ]; then
    awk -v start="$start_marker" -v end="$end_marker" '
        $0 == start { skip = 1; next }
        $0 == end { skip = 0; next }
        !skip { lines[++count] = $0 }
        END {
            while (count > 0 && lines[count] == "") {
                count--
            }
            for (i = 1; i <= count; i++) {
                print lines[i]
            }
        }
    ' "$target_file" > "$tmp_file"

    if [ -s "$tmp_file" ]; then
        printf '\n' >> "$tmp_file"
    fi
else
    cat <<'EOF' > "$tmp_file"
# Machine-specific zsh login-shell customizations belong here.
# Use this for exported environment variables and PATH changes that must exist
# before the shared ~/.zshrc runs.
EOF
    printf '\n' >> "$tmp_file"
fi

managed_block >> "$tmp_file"
mv "$tmp_file" "$target_file"
trap - EXIT HUP INT TERM

log "ensured Homebrew shellenv block in $target_file"
