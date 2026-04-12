#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$script_dir/lib/common.sh"

ANTIDOTE_DIR=${ANTIDOTE_DIR:-$HOME/.antidote}
ANTIDOTE_REPO_URL=${ANTIDOTE_REPO_URL:-https://github.com/mattmc3/antidote.git}

if [ -d "$ANTIDOTE_DIR/.git" ]; then
    log "updating antidote at $ANTIDOTE_DIR"
    git -C "$ANTIDOTE_DIR" pull --ff-only
elif [ -e "$ANTIDOTE_DIR" ]; then
    die "$ANTIDOTE_DIR exists but is not an antidote git checkout"
else
    log "installing antidote at $ANTIDOTE_DIR"
    git clone --depth=1 "$ANTIDOTE_REPO_URL" "$ANTIDOTE_DIR"
fi
