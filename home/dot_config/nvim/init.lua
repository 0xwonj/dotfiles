vim.g.mapleader = " "
vim.g.maplocalleader = "\\"
vim.g.have_nerd_font = false
vim.g.loaded_node_provider = 0
vim.g.loaded_perl_provider = 0
vim.g.loaded_ruby_provider = 0

if vim.loader then
  vim.loader.enable()
end

require("config.options")
require("config.keymaps")
require("config.autocmds")
require("config.lazy")
