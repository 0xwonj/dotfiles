#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$script_dir/lib/common.sh"

required_commands="git stow zsh curl"
recommended_commands="git-lfs tmux nvim starship gh fzf zoxide eza btop fastfetch yazi"

missing_required=0

print_group() {
    label=$1
    shift

    printf '%s\n' "$label"
    for cmd in "$@"; do
        if have_cmd "$cmd"; then
            printf '  ok   %-12s %s\n' "$cmd" "$(command -v "$cmd")"
        else
            printf '  miss %-12s\n' "$cmd"
        fi
    done
}

for cmd in $required_commands; do
    if ! have_cmd "$cmd"; then
        missing_required=1
    fi
done

# intentional word splitting for command groups
# shellcheck disable=SC2086
print_group "Required commands" $required_commands
printf '\n'
# shellcheck disable=SC2086
print_group "Recommended commands" $recommended_commands
printf '\n'

if [ -f "$HOME/.antidote/antidote.zsh" ]; then
    printf '  ok   %-12s %s\n' "antidote" "$HOME/.antidote/antidote.zsh"
else
    printf '  miss %-12s %s\n' "antidote" "~/.antidote/antidote.zsh"
fi

exit "$missing_required"
