local bootstrap = require("config.bootstrap")

return {
  {
    "nvim-treesitter/nvim-treesitter",
    lazy = false,
    build = function()
      require("nvim-treesitter").install(bootstrap.treesitter_languages):wait(300000)
    end,
    opts = {
      install_dir = vim.fn.stdpath("data") .. "/site",
    },
    config = function(_, opts)
      require("nvim-treesitter").setup(opts)
      vim.treesitter.language.register("json", "jsonc")

      local group = vim.api.nvim_create_augroup("treesitter-enable", { clear = true })
      vim.api.nvim_create_autocmd("FileType", {
        group = group,
        pattern = "*",
        callback = function(args)
          local ok = pcall(vim.treesitter.start, args.buf)
          if ok and vim.bo[args.buf].filetype ~= "markdown" then
            vim.bo[args.buf].indentexpr = "v:lua.require'nvim-treesitter'.indentexpr()"
          end
        end,
      })
    end,
  },
}
