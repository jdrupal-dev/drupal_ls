# Drupal LS
The missing language server for Drupal.

## Features
<details>

<summary>Hover</summary>

- Service references
- Service class
- Route references
- Route controller/form
- Hook references
- Permission references
- Plugin references

</details>
<details>

<summary>Go to definition</summary>

- Service references
- Service class
- Route references
- Route controller/form
- Hook references
- Permission references
- Plugin references

</details>
<details>

<summary>Completion</summary>

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

</details>
<details>

<summary>Code actions</summary>

- Add translation placeholders to `t()` functions.

</details>

## Installation
<details>

<summary>VSCode</summary>

You can download the VSCode extension by searching for `drupal-ls` in VSCode or going to [the marketplace](https://marketplace.visualstudio.com/items?itemName=jdrupal-dev.drupal-ls).

Currently the extension is supported on the following platforms:
- MacOS (darwin-x64, darwin-arm64)
- Linux (linux-x64, linux-arm64)

</details>

<details>

<summary>Neovim (lazy.nvim)</summary>

You can download a pre-built binary from the Releases page, or you can compile it from source.

```lua
{
  "jdrupal-dev/drupal_ls",
  event = { "BufReadPre", "BufNewFile" },
  -- Requires cargo to be installed locally.
  -- Only needed when compiling from source.
  build = "cargo build --release",
  config = function()
    vim.lsp.config.drupal_ls = {
      cmd = {
        -- Replace this path, if you download a prebuilt binary.
        vim.fn.stdpath("data") .. "/lazy/drupal_ls/target/release/drupal_ls",
        "--file",
        "/tmp/drupal_ls-log.txt",
      },
      filetypes = { "php", "yaml" },
      root_markers = {
        'composer.json',
      },
    };

    vim.lsp.enable("drupal_ls");
  end,
}
```

</details>

## Roadmap
### VSCode
- [ ] Build VSCode extention in Ci.

### Completion
- [ ] Autocomplete #theme functions.

### Code actions
- [ ] Generate __construct doc block.
