### run
```bash
trunk serve --open
```
### update styles
```bash
npx tailwindcss -i ./input.css -o ./public/typing.css --watch
```
### format letptos code
```bash
leptosfmt .
```
### run translation server

using https://huggingface.co/docs/transformers/model_doc/marian

#### dependencies

AWS EC2

use 16GB of storage (after the installation uses 7.8GB of store, during the installation up to 12GB)

use t2.medium to install and t2.small to run

```python
sudo yum install python pip
mkdir tmp
TMPDIR=/home/ec2-user/tmp pip install torch flask transformers sentencepiece sacremoses --no-cache-dir
```

```python
python translate_server.py
```

### nvim init.lua config:

```lua
      local servers = {
        -- clangd = {},
        -- gopls = {},
        -- pyright = {},
        -- rust_analyzer = {},
        -- ... etc. See `:help lspconfig-all` for a list of all the pre-configured LSPs
        --
        -- Some languages (like typescript) have entire language plugins that can be useful:
        --    https://github.com/pmizio/typescript-tools.nvim
        --
        -- But for many setups, the LSP (`tsserver`) will work just fine
        -- tsserver = {},
        --

        rust_analyzer = {
          settings = {
            ['rust-analyzer'] = {
              cargo = {
                allFeatures = true,
                -- features = { 'ssr' }, -- features = ssr, for LSP support in leptos SSR functions
              },
            },
          },
        },
        lua_ls = {

```
