if [[ -f "$HOME/.config/shell/env.sh" ]]; then
  source "$HOME/.config/shell/env.sh"
fi

if [[ -f "$HOME/.config/shell/path.sh" ]]; then
  source "$HOME/.config/shell/path.sh"
fi

typeset -U path PATH

if [[ -f "$HOME/.zprofile.local" ]]; then
  source "$HOME/.zprofile.local"
  typeset -U path PATH
fi
