local map = vim.keymap.set

map("n", "<Esc>", "<cmd>nohlsearch<CR>")

map("n", "<leader>w", "<cmd>write<CR>", { desc = "Write" })
map("n", "<leader>q", "<cmd>quit<CR>", { desc = "Quit" })
map("n", "<leader>Q", "<cmd>qa!<CR>", { desc = "Quit all" })

map("n", "<C-h>", "<C-w><C-h>", { desc = "Move focus left" })
map("n", "<C-j>", "<C-w><C-j>", { desc = "Move focus down" })
map("n", "<C-k>", "<C-w><C-k>", { desc = "Move focus up" })
map("n", "<C-l>", "<C-w><C-l>", { desc = "Move focus right" })

map("n", "<leader>-", "<cmd>split<CR>", { desc = "Split below" })
map("n", '<leader>|', "<cmd>vsplit<CR>", { desc = "Split right" })

map("n", "<leader>cd", vim.diagnostic.open_float, { desc = "Line diagnostics" })

map("t", "<Esc><Esc>", "<C-\\><C-n>", { desc = "Exit terminal mode" })
