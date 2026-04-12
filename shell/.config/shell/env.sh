: "${EDITOR:=}"
if [ -z "$EDITOR" ]; then
    if command -v nvim >/dev/null 2>&1; then
        EDITOR=nvim
    elif command -v vim >/dev/null 2>&1; then
        EDITOR=vim
    else
        EDITOR=vi
    fi
fi
: "${VISUAL:=$EDITOR}"
: "${PAGER:=less -FRX}"
: "${GOPATH:=$HOME/go}"
: "${VIRTUAL_ENV_DISABLE_PROMPT:=1}"
export EDITOR VISUAL PAGER GOPATH VIRTUAL_ENV_DISABLE_PROMPT
