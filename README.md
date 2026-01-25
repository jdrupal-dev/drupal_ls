# Drupal LS
The missing language server for Drupal.

## Installation

### VSCode

You can download the VSCode extension by searching for `drupal-ls` in VSCode or going to [the marketplace](https://marketplace.visualstudio.com/items?itemName=jdrupal-dev.drupal-ls).

Currently the extension is supported on the following platforms:
- MacOS (darwin-x64, darwin-arm64)
- Linux (linux-x64, linux-arm64)

### Neovim (lazy.nvim)

#### Option 1: Use Pre-built Binary

1. Download the binary for your system from the **Releases** page.
2. Move it to a directory in your system `PATH` (e.g., `/usr/local/bin`) and ensure it is executable.

```bash
# Example: Move the downloaded binary to /usr/local/bin
mv ~/Downloads/drupal_ls /usr/local/bin/drupal_ls
chmod +x /usr/local/bin/drupal_ls
```

3. Update your `lazy.nvim` configuration:

```lua
{
  "jdrupal-dev/drupal_ls",
  event = { "BufReadPre", "BufNewFile" },
  config = function()
    vim.lsp.config.drupal_ls = {
      cmd = {
        -- If the binary is in your PATH, you can just use the command name:
        "drupal_ls",
        -- Or specify the absolute path:
        -- "/usr/local/bin/drupal_ls",
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

#### Option 2: Compile from Source

**Prerequisites:** You must have **Rust** and **Cargo** installed locally and globally accessible.
[Install Rust and Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)

```lua
{
  "jdrupal-dev/drupal_ls",
  event = { "BufReadPre", "BufNewFile" },
  -- Requires cargo to be installed locally.
  build = "cargo build --release",
  config = function()
    vim.lsp.config.drupal_ls = {
      cmd = {
        -- The binary is built in the plugin directory
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

## Roadmap
### VSCode
- [ ] Build VSCode extension in CLI automatically.

### Completion
- [ ] Autocomplete #theme functions in render arrays.

### Code actions
- [ ] Generate __construct doc block for classes.
