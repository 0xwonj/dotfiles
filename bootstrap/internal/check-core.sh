#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$script_dir/lib/common.sh"

prepend_user_bins
prepend_brew_path_if_present

required_commands="git stow zsh curl nvim"
convenience_commands="fzf zoxide eza"
missing_required=0
missing_convenience=0
required_total=0
convenience_total=0
pm=$(detect_package_manager)

print_group() {
    label=$1
    shift

    printf '%s\n' "$label"
    for cmd in "$@"; do
        if have_cmd "$cmd"; then
            status_line ok "$(printf '%-12s %s' "$cmd" "$(command -v "$cmd")")"
        else
            status_line miss "$cmd"
        fi
    done
}

for cmd in $required_commands; do
    required_total=$((required_total + 1))
    if ! have_cmd "$cmd"; then
        missing_required=$((missing_required + 1))
    fi
done

for cmd in $convenience_commands; do
    convenience_total=$((convenience_total + 1))
    if ! have_cmd "$cmd"; then
        missing_convenience=$((missing_convenience + 1))
    fi
done

detail_line "package mgr" "$pm"
printf '\n'

# intentional word splitting for command groups
# shellcheck disable=SC2086
print_group "Required commands" $required_commands
printf '\n'
# intentional word splitting for command groups
# shellcheck disable=SC2086
print_group "Default convenience commands" $convenience_commands
printf '\n'

if [ -f "$HOME/.antidote/antidote.zsh" ]; then
    status_line ok "$(printf '%-12s %s' "antidote" "$HOME/.antidote/antidote.zsh")"
else
    status_line note "antidote     ~/.antidote/antidote.zsh"
fi

printf '\n'
detail_line required "$(printf '%s/%s ok' "$((required_total - missing_required))" "$required_total")"
detail_line convenience "$(printf '%s/%s ok' "$((convenience_total - missing_convenience))" "$convenience_total")"

if [ "$missing_required" -gt 0 ]; then
    exit 1
fi

exit 0
