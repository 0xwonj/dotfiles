PATH=${PATH:-}

path_remove() {
    [ $# -eq 1 ] || return 0
    target=$1
    [ -n "$target" ] || return 0

    old_ifs=$IFS
    IFS=:
    new_path=
    for entry in $PATH; do
        [ -n "$entry" ] || continue
        [ "$entry" = "$target" ] && continue
        if [ -n "$new_path" ]; then
            new_path="${new_path}:$entry"
        else
            new_path=$entry
        fi
    done
    IFS=$old_ifs
    PATH=$new_path
}

path_prepend() {
    [ $# -eq 1 ] || return 0
    dir=$1
    [ -n "$dir" ] || return 0
    [ -d "$dir" ] || return 0

    case ":$PATH:" in
        *:"$dir":*) ;;
        *) PATH="$dir${PATH:+:$PATH}" ;;
    esac
}

path_dedupe() {
    old_ifs=$IFS
    IFS=:
    new_path=
    for entry in $PATH; do
        [ -n "$entry" ] || continue
        case ":$new_path:" in
            *:"$entry":*) ;;
            *)
                if [ -n "$new_path" ]; then
                    new_path="${new_path}:$entry"
                else
                    new_path=$entry
                fi
                ;;
        esac
    done
    IFS=$old_ifs
    PATH=$new_path
}

source_if_exists() {
    [ $# -eq 1 ] || return 0
    [ -f "$1" ] && . "$1"
}

path_prepend "$HOME/.local/bin"
path_prepend "${GOPATH:-$HOME/go}/bin"
path_prepend "$HOME/.foundry/bin"
path_prepend "$HOME/.sp1/bin"
path_prepend "$HOME/.risc0/bin"

source_if_exists "$HOME/.cargo/env"

path_dedupe
export PATH
