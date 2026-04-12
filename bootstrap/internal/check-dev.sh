#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$script_dir/lib/common.sh"

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

prepend_user_bins
prepend_brew_path_if_present

python_commands="uv"
rust_commands="rustup cargo rustc"
javascript_commands="node npm"
agent_commands="codex"
missing=0

for cmd in $python_commands $rust_commands $javascript_commands $agent_commands; do
    if ! have_cmd "$cmd"; then
        missing=1
    fi
done

# intentional word splitting for command groups
# shellcheck disable=SC2086
print_group "Python tooling" $python_commands
printf '\n'
# shellcheck disable=SC2086
print_group "Rust tooling" $rust_commands
printf '\n'
# shellcheck disable=SC2086
print_group "JavaScript tooling" $javascript_commands
printf '\n'
# shellcheck disable=SC2086
print_group "Agent CLIs" $agent_commands

exit "$missing"
