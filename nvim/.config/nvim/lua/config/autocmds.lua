local group = vim.api.nvim_create_augroup("user-config", { clear = true })

vim.api.nvim_create_autocmd("TextYankPost", {
  group = group,
  desc = "Highlight when yanking text",
  callback = function()
    vim.highlight.on_yank()
  end,
})

vim.api.nvim_create_autocmd("BufReadPost", {
  group = group,
  desc = "Restore cursor position",
  callback = function(args)
    local excluded = {
      gitcommit = true,
      help = true,
    }
    if excluded[vim.bo[args.buf].filetype] then
      return
    end

    local line = vim.api.nvim_buf_get_mark(args.buf, '"')[1]
    if line > 0 and line <= vim.api.nvim_buf_line_count(args.buf) then
      pcall(vim.api.nvim_win_set_cursor, 0, { line, 0 })
    end
  end,
})

vim.api.nvim_create_autocmd("FileType", {
  group = group,
  pattern = { "gitcommit", "help", "markdown", "text" },
  desc = "Improve editing experience for prose-like buffers",
  callback = function()
    vim.opt_local.wrap = true
    vim.opt_local.linebreak = true
    vim.opt_local.spell = true
  end,
})
