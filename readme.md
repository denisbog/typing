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
leptosfmt src/
```

### run translation server

using `https://huggingface.co/docs/transformers/model_doc/marian`

#### dependencies

AWS EC2

use 16GB of storage (after the installation uses 7.8GB of store, during the installation up to 12GB)

use t2.medium to install and t2.small to run

```python
sudo yum install python pip
mkdir tmp
TMPDIR=/home/ec2-user/tmp pip install torch flask transformers sentencepiece sacremoses --no-cache-dir
```

### start translation server

```python
python translate_server.py
```

### translate articles

```bash
time cargo r --bin translate --release --features=translation
```

### translate with rust ML

```bash
time PATH=$PATH:/usr/local/cuda-12.5/bin/ cargo run --release --features translation --bin translation-tool --  --tokenizer tokenizer-marian-base-de.json --tokenizer-dec tokenizer-marian-base-en.json
time NVCC_CCBIN=gcc-13 PATH=$PATH:/usr/local/cuda/bin cargo run --release --features translation --bin translation-tool
```

### nvim init.lua config

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

### build for AWS image

```docker
docker build -t build . -f container/Dockerfile
```

### build and deploy as AWS Lambda

passing additinal feature `aws`, LEPTOS_OUTPUT_NAME=`typing` should match the main artifact from `Cargo.toml`

## build and deploy the application

```bash
npx tailwindcss -i ./input.css -o ./public/typing.css
cargo leptos watch --release
npx tailwindcss -i ./input.css -o ./public/typing.css
# RUSTFLAGS="-Zlinker-features=-lld" LEPTOS_OUTPUT_NAME=typing cargo lambda build --no-default-features --features=ssr,lambda --release

npx tailwindcss -i ./input.css -o ./public/typing.css
LEPTOS_OUTPUT_NAME=typing cargo lambda build --no-default-features --features=ssr,lambda --release
cargo lambda deploy --include target/site --enable-function-url --binary-name=typing

## translation
```bash
NVCC_CCBIN=gcc-14 PATH=$PATH:/usr/local/cuda/bin cargo r --release --package translation-tool
NVCC_CCBIN=gcc-14 PATH=$PATH:/usr/local/cuda/bin cargo r --release --package spiegel-crawler --bin spiegel-crawler -- --userInfo  <userInfo> --accessInfo <accessInfo> --userId <userId>
NVCC_CCBIN=gcc-14 PATH=$PATH:/usr/local/cuda-12/bin cargo r --release --package translation-tool
sudo dkms install --force nvidia/580.105.08 -k $(uname -r)
PATH=$PATH:/usr/local/cuda/bin cargo r --release --package translation-tool
sudo ln -sf /usr/lib64/libcuda.so.1 /usr/lib/libcuda.so.1
sudo ln -sf /usr/lib64/libcuda.so.1 /usr/lib/libcuda.so
```


## delete from command line

```sh
aws dynamodb delete-item --table-name translation --key file://id.json
```

id.json

```json
{
  "user_id": {
    "S": "e4e8e4c8-c0e1-7060-0062-*****"
  },
  "created_at": {
    "N": "1764090714075"
  }
}
```
