return {
  {
    "tpope/vim-fugitive",
    cmd = { "Git", "Gdiffsplit", "Gvdiffsplit" },
    keys = {
      { "<leader>gg", "<cmd>Git<CR>", desc = "Git status" },
    },
  },
  {
    "lewis6991/gitsigns.nvim",
    event = { "BufReadPre", "BufNewFile" },
    opts = {
      signs = {
        add = { text = "+" },
        change = { text = "~" },
        delete = { text = "_" },
        topdelete = { text = "^" },
        changedelete = { text = "~" },
      },
      on_attach = function(bufnr)
        local gs = package.loaded.gitsigns

        local function map(lhs, rhs, desc, mode)
          vim.keymap.set(mode or "n", lhs, rhs, {
            buffer = bufnr,
            desc = desc,
          })
        end

        map("]h", gs.next_hunk, "Next hunk")
        map("[h", gs.prev_hunk, "Previous hunk")
        map("<leader>gp", gs.preview_hunk, "Preview hunk")
        map("<leader>gr", gs.reset_hunk, "Reset hunk")
        map("<leader>gR", gs.reset_buffer, "Reset buffer")
        map("<leader>ghs", gs.stage_hunk, "Stage hunk")
        map("<leader>ghS", gs.stage_buffer, "Stage buffer")
        map("<leader>gb", function()
          gs.blame_line({ full = true })
        end, "Blame line")
      end,
    },
  },
}
