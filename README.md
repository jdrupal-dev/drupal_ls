# Drupal LS
The missing language server for Drupal.

## Features
### Hover
- Service references
- Service class
- Route references
- Route controller/form
- Hook references
- Permission references
- Plugin references
### Go to definition
- Service references
- Service class
- Route references
- Route controller/form
- Hook references
- Permission references
- Plugin references
### Completion
- Services
- Routes
- Snippets
    - A few QoL improving snippets.
    - Hooks
    - form-[ELEMENT]
    - render-[ELEMENT]
- Permissions
- Plugin IDs (limited to:)
    - EntityType
    - QueueWorker
    - FieldType
    - DataType
    - FormElement
    - RenderElement
### Code actions
- Add translation placeholders to `t()` functions.

## Installation

### VSCode extension available in marketplace
You can download the VSCode extension by searching for `drupal-ls` in VSCode or going to [the marketplace](https://marketplace.visualstudio.com/items?itemName=jdrupal-dev.drupal-ls).

Currently the extension is supported on the following platforms:
- MacOS (darwin-x64, darwin-arm64)
- Linux (linux-x64, linux-arm64)

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
### VSCode
- [ ] Build VSCode extention in Ci.

### Completion
- [ ] Autocomplete #theme functions.

### Code actions
- [ ] Generate __construct doc block.
