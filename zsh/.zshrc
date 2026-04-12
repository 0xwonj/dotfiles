[[ $- != *i* ]] && return

mkdir -p "$HOME/.cache/zsh"

HISTFILE="$HOME/.zsh_history"
HISTSIZE=100000
SAVEHIST=100000
setopt APPEND_HISTORY
setopt INC_APPEND_HISTORY
setopt HIST_IGNORE_DUPS

# Antidote static bundle
zsh_plugins_base="${ZDOTDIR:-$HOME}/.zsh_plugins"
if [[ -f "${zsh_plugins_base}.txt" ]]; then
  if [[ ! -f "${zsh_plugins_base}.zsh" || "${zsh_plugins_base}.zsh" -ot "${zsh_plugins_base}.txt" ]]; then
    if [[ -f "$HOME/.antidote/antidote.zsh" ]]; then
      source "$HOME/.antidote/antidote.zsh"
      antidote bundle <"${zsh_plugins_base}.txt" >"${zsh_plugins_base}.zsh"
    fi
  fi

  if [[ -f "${zsh_plugins_base}.zsh" ]]; then
    source "${zsh_plugins_base}.zsh"
  fi
fi

autoload -Uz compinit
compinit -d "$HOME/.cache/zsh/zcompdump-$ZSH_VERSION"

if (( $+commands[fzf] )); then
  source <(fzf --zsh)
fi

if (( $+commands[zoxide] )); then
  eval "$(zoxide init zsh)"
fi

if (( $+commands[starship] )); then
  eval "$(starship init zsh)"
fi

if (( $+commands[eza] )); then
  alias l='eza --group-directories-first --icons=auto'
  alias ll='eza -lh --group-directories-first --icons=auto'
  alias la='eza -lah --group-directories-first --icons=auto'
  alias lt='eza --tree --level=2 --icons=auto'
fi

if (( $+commands[python3.14] )); then
  alias python='python3.14'
fi

if [[ -f "$HOME/.zshrc.local" ]]; then
  source "$HOME/.zshrc.local"
fi
