#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$script_dir/lib/common.sh"

prepend_user_bins
prepend_brew_path_if_present

required_commands="git stow zsh curl"
missing_required=0
missing_recommended=0
required_total=0
recommended_total=0
pm=$(detect_package_manager)

recommended_commands_for_pm() {
    case $1 in
        brew)
            printf '%s\n' "git-lfs tmux nvim starship gh fzf zoxide eza btop yazi"
            ;;
        apt | dnf | pacman)
            printf '%s\n' "git-lfs tmux nvim gh fzf zoxide eza btop yazi"
            ;;
        *)
            printf '%s\n' "git-lfs tmux nvim gh fzf zoxide eza btop yazi"
            ;;
    esac
}

recommended_commands=$(recommended_commands_for_pm "$pm")
extra_commands=

case $pm in
    brew)
        ;;
    *)
        extra_commands="starship"
        ;;
esac

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

for cmd in $recommended_commands; do
    recommended_total=$((recommended_total + 1))
    if ! have_cmd "$cmd"; then
        missing_recommended=$((missing_recommended + 1))
    fi
done

detail_line "package mgr" "$pm"
printf '\n'

# intentional word splitting for command groups
# shellcheck disable=SC2086
print_group "Required commands" $required_commands
printf '\n'
# shellcheck disable=SC2086
print_group "Recommended commands" $recommended_commands
printf '\n'

if [ -n "$extra_commands" ]; then
    # shellcheck disable=SC2086
    print_group "Extra commands" $extra_commands
    printf '\n'
fi

if [ -f "$HOME/.antidote/antidote.zsh" ]; then
    status_line ok "$(printf '%-12s %s' "antidote" "$HOME/.antidote/antidote.zsh")"
else
    status_line miss "antidote     ~/.antidote/antidote.zsh"
fi

printf '\n'
detail_line required "$(printf '%s/%s ok' "$((required_total - missing_required))" "$required_total")"
detail_line recommended "$(printf '%s/%s ok' "$((recommended_total - missing_recommended))" "$recommended_total")"

if [ "$missing_required" -gt 0 ]; then
    exit 1
fi

exit 0
