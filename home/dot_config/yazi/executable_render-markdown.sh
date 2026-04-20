#!/bin/sh
set -eu

mode=${1:-}
file=${2:-}

if [ -z "$mode" ] || [ -z "$file" ]; then
    printf 'usage: %s <preview|full> <file> [width]\n' "$0" >&2
    exit 1
fi

have_glow() {
    command -v glow >/dev/null 2>&1
}

glow_style() {
    printf '%s\n' "${GLOW_STYLE:-tokyo-night}"
}

preview_markdown() {
    width=${1:-80}
    style=$(glow_style)

    if have_glow; then
        CLICOLOR_FORCE=1 glow -s "$style" -w "$width" "$file"
        return
    fi

    exec bat \
        --paging=never \
        --color=always \
        --style=plain \
        --language=md \
        --wrap=character \
        --terminal-width="$width" \
        "$file"
}

full_markdown() {
    style=$(glow_style)

    if have_glow; then
        exec glow -t -s "$style" -w "${GLOW_FULL_WIDTH:-92}" "$file"
    fi

    exec bat \
        --paging=always \
        --color=always \
        --language=md \
        "$file"
}

case "$mode" in
    preview)
        preview_markdown "${3:-80}"
        ;;
    full)
        full_markdown
        ;;
    *)
        printf 'unknown mode: %s\n' "$mode" >&2
        exit 1
        ;;
esac
