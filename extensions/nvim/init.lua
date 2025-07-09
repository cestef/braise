local M = {}

function M.setup(opts)
  opts = opts or {}
  
  -- Filetype detection
  vim.filetype.add({
    extension = {
      braise = "braise",
      braisefile = "braise",
    },
    filename = {
      Braisefile = "braise",
    },
  })
  
  -- LSP configuration
  local lspconfig = require('lspconfig')
  local configs = require('lspconfig.configs')
  
  if not configs.braise then
    configs.braise = {
      default_config = {
        cmd = opts.cmd or { 'braise', 'lsp' },
        filetypes = { 'braise' },
        root_dir = lspconfig.util.root_pattern('Braisefile', '.braise'),
        settings = opts.settings or {},
      },
    }
  end
  
  lspconfig.braise.setup(vim.tbl_deep_extend('force', {
    on_attach = function(client, bufnr)
      local bufopts = { noremap = true, silent = true, buffer = bufnr }
      vim.keymap.set('n', 'gd', vim.lsp.buf.definition, bufopts)
      vim.keymap.set('n', 'K', vim.lsp.buf.hover, bufopts)
      vim.keymap.set('n', '<space>rn', vim.lsp.buf.rename, bufopts)
      vim.keymap.set('n', '<space>ca', vim.lsp.buf.code_action, bufopts)
      vim.keymap.set('n', 'gr', vim.lsp.buf.references, bufopts)
      vim.keymap.set('n', '<space>f', function()
        vim.lsp.buf.format { async = true }
      end, bufopts)
    end,
  }, opts))
  
  -- File type settings
  vim.api.nvim_create_autocmd('FileType', {
    pattern = 'braise',
    callback = function()
      vim.bo.commentstring = '// %s'
      vim.bo.shiftwidth = 2
      vim.bo.tabstop = 2
      vim.bo.expandtab = true
    end,
  })
end

return M