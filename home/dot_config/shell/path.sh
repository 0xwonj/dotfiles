PATH=${PATH:-}

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

path_prepend "/opt/homebrew/bin"
path_prepend "/opt/homebrew/sbin"
path_prepend "/usr/local/bin"
path_prepend "/usr/local/sbin"
path_prepend "/home/linuxbrew/.linuxbrew/bin"
path_prepend "/home/linuxbrew/.linuxbrew/sbin"
path_prepend "$HOME/.risc0/bin"
path_prepend "$HOME/.sp1/bin"
path_prepend "$HOME/.foundry/bin"
path_prepend "${GOPATH:-$HOME/go}/bin"
path_prepend "$HOME/.cargo/bin"
path_prepend "$HOME/.local/bin"

path_dedupe
export PATH
