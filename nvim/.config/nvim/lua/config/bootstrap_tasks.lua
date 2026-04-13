local bootstrap = require("config.bootstrap")

local M = {}

local function write_words(items)
  io.write(table.concat(items, " "))
end

function M.print_mason_packages()
  write_words(bootstrap.mason_packages)
end

function M.print_treesitter_languages()
  write_words(bootstrap.treesitter_languages)
end

local function get_missing_mason_package_names(registry)
  local missing = {}
  for _, name in ipairs(bootstrap.mason_packages) do
    local ok, pkg = pcall(registry.get_package, name)
    assert(ok and pkg, ("Unknown Mason package: %s"):format(name))
    if not pkg:is_installed() then
      table.insert(missing, name)
    end
  end

  return missing
end

function M.print_missing_mason_packages()
  local registry = require("mason-registry")
  registry.refresh()
  write_words(get_missing_mason_package_names(registry))
end

local function install_mason_package(pkg)
  local done = false
  local success = false
  local err

  pkg:install({}, function(ok, result)
    success = ok
    err = result
    done = true
  end)

  local completed = vim.wait(300000, function()
    return done
  end, 50)

  assert(completed, ("Timed out installing Mason package: %s"):format(pkg.name))
  assert(success, ("Failed installing Mason package %s: %s"):format(pkg.name, tostring(err)))
end

function M.ensure_mason_packages(opts)
  opts = opts or {}

  local registry = require("mason-registry")
  registry.refresh()

  local packages = opts.update and bootstrap.mason_packages or get_missing_mason_package_names(registry)
  for _, name in ipairs(packages) do
    install_mason_package(registry.get_package(name))
  end
end

function M.ensure_treesitter_parsers()
  require("nvim-treesitter").install(bootstrap.treesitter_languages):wait(300000)
end

return M
