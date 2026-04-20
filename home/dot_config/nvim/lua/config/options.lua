local opt = vim.opt

opt.number = true
opt.relativenumber = true
opt.cursorline = true
opt.mouse = "a"
opt.showmode = false
opt.clipboard = "unnamedplus"
opt.breakindent = true
opt.undofile = true
opt.ignorecase = true
opt.smartcase = true
opt.signcolumn = "yes"
opt.updatetime = 250
opt.timeoutlen = 300
opt.splitright = true
opt.splitbelow = true
opt.inccommand = "split"
opt.confirm = true
opt.scrolloff = 8
opt.sidescrolloff = 8
opt.wrap = false
opt.expandtab = true
opt.tabstop = 2
opt.softtabstop = 2
opt.shiftwidth = 2
opt.smartindent = true
opt.termguicolors = true
opt.list = true
opt.listchars = {
  tab = "> ",
  trail = ".",
  nbsp = "_",
}
opt.fillchars = {
  eob = " ",
}
opt.laststatus = 3
opt.completeopt = {
  "menu",
  "menuone",
  "noselect",
  "popup",
}
opt.pumheight = 10
opt.winborder = "rounded"
opt.conceallevel = 0

if vim.fn.executable("rg") == 1 then
  opt.grepprg = "rg --vimgrep --smart-case --hidden"
  opt.grepformat = "%f:%l:%c:%m"
end
