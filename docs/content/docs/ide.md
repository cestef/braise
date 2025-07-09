+++
title = "IDE Support"
weight = 10
+++

The Braisé CLI includes a built-in language server (LSP) that provides basic diagnostics and formatting support for Braisé files. 

This allows you to use Braisé in your favorite IDEs that support LSP, such as Visual Studio Code, Neovim, and others.

```bash, copy
braise lsp # will start the LSP
```

## Visual Studio Code

A basic extension for Visual Studio Code is available to provide syntax highlighting and LSP support. You can install it from the [Visual Studio Code Marketplace](https://marketplace.visualstudio.com/items?itemName=cstef.braise).

## Neovim


> [!WARNING]
> Syntax highlighting is not supported at the moment for nvim.

For Neovim, you can use the built-in LSP client to connect to the Braisé LSP. You can add the following configuration to your `init.lua`:

```lua, copy, name=init.lua
-- Ensure you have the lspconfig plugin installed
require("lspconfig").braise.setup {
    cmd = {"braise", "lsp"},
    filetypes = {"braise"},
    root_dir = function(fname)
        return vim.fn.getcwd()
    end,
    settings = {
        braise = {
            diagnostics = {
                enable = true
            },
            formatting = {
                enable = true
            }
        }
    }
}
```

## Other IDEs

You can use the Braisé LSP with any IDE that supports LSP. The configuration will be similar to the Neovim example above. You will need to specify the command to start the Braisé LSP server, the file types, and any additional settings you want to configure.

Syntax highlighting can be achieved using either `braise.sublime-syntax` for Sublime Text or `braise.tmLanguage.json` for other editors that support TextMate grammars. These are located under the [`syntaxes`](https://github.com/cestef/braise/tree/dev/syntaxes) directory in the repository.