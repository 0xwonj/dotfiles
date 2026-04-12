local opt = vim.opt

opt.number = true
opt.relativenumber = true
opt.cursorline = true

opt.ignorecase = true
opt.smartcase = true
opt.hlsearch = true
opt.incsearch = true

opt.expandtab = true
opt.shiftwidth = 2
opt.softtabstop = 2
opt.smartindent = true

opt.wrap = false
opt.scrolloff = 5
opt.sidescrolloff = 5
opt.showbreak = "↪ "

opt.list = true
opt.listchars = {
  tab = "▸ ",
  trail = "·",
  extends = "…",
  precedes = "…",
}
opt.fillchars = {
  vert = "│",
  horiz = "─",
  eob = " ",
}

opt.conceallevel = 2
opt.concealcursor = "nc"
opt.clipboard = "unnamedplus"
opt.mouse = "a"

opt.undofile = true
opt.backup = false
opt.writebackup = false

opt.termguicolors = true
opt.signcolumn = "yes"
opt.showmode = false
opt.cmdheight = 1
opt.completeopt = { "menuone", "noselect" }
opt.laststatus = 2
opt.statusline = "%#StatusLine# %f %m%r %#StatusLineNC#%=%#StatusLine#  L:%l C:%c "

vim.api.nvim_set_hl(0, "StatusLine", { bg = "#3E4452", fg = "#ABB2BF" })
vim.api.nvim_set_hl(0, "StatusLineNC", { bg = "#282C34", fg = "#5C6370" })
vim.api.nvim_set_hl(0, "VertSplit", { fg = "#4B5263" })

local ok = pcall(vim.cmd.colorscheme, "dracula")
if not ok then
  vim.cmd.colorscheme("habamax")
end

vim.api.nvim_create_autocmd("BufWritePre", {
  pattern = "*",
  callback = function()
    vim.cmd([[%s/\s\+$//e]])
  end,
})

vim.cmd("filetype plugin indent on")
