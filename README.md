# Drupal LS
The missing language server for Drupal.

WIP!

## Features
### Hover
- Service references
- Service class
- Route references
- Route controller/form
- Hook references
- Permission references
### Go to definition
- Service references
- Service class
- Route references
- Route controller/form
- Hook references
- Permission references
### Completion
- Services
- Routes
- Hook snippets
- General snippets
- Permissions

## Installation

### Neovim installation with [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig) and lazy.nvim

```lua
{
  "jdrupal-dev/drupal_ls",
  event = { "BufReadPre", "BufNewFile" },
  dependencies = "neovim/nvim-lspconfig",
  -- Requires cargo to be installed locally.
  build = "cargo build --release",
  config = function()
    local lspconfig = require("lspconfig")

    require("lspconfig.configs").drupal_ls = {
      default_config = {
        cmd = {
          vim.fn.stdpath("data") .. "/lazy/drupal_ls/target/release/drupal_ls",
          "--file",
          "/tmp/drupal_ls-log.txt",
        },
        filetypes = { "php", "yaml" },
        root_dir = lspconfig.util.root_pattern("composer.json"),
        settings = {},
      },
    }

    lspconfig["drupal_ls"].setup({})
  end,
}
```

## Roadmap
### Completion
- [ ] Autocomplete plugin IDs (eg. queue workers, blocks, fields, migrate source/process/destination).
- [ ] Autocomplete #theme functions.

### Code actions
- [ ] Generate __construct doc block.
- [ ] Generate t function placeholder array.
