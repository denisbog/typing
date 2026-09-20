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

cargo clean

npx tailwindcss -i ./input.css -o ./public/typing.css
# TYPING_BUILD stamps BUILD_NUMBER and the PWA cache name (build.rs ->
# public/build.json + public/sw.js) so both change on every deployment.
# Without it, the service worker cache name stays the same and clients keep
# the old cached build.
TYPING_BUILD=$(date -u +%s) cargo leptos build --release
npx tailwindcss -i ./input.css -o ./public/typing.css
cargo leptos build --release
LEPTOS_OUTPUT_NAME=typing cargo lambda build --no-default-features --features=ssr,lambda --release
cargo lambda deploy --include target/site --enable-function-url --binary-name=typing
```

## translation

Project-local .cargo/config.toml in this repo:

 ```toml
   [env]
   NVCC_CCBIN = "gcc-14"
   NVCC_PREPEND_FLAGS = "-U_GNU_SOURCE -D_DEFAULT_SOURCE"
 ```

The tool needs the converted Marian tokenizers (`tokenizer-marian-base-de.json`,
`tokenizer-marian-base-en.json`) in the directory it is run from. They are
generated with candle's marian-mt `convert_slow_tokenizer.py` from the
Helsinki-NLP/opus-mt-de-en sentencepiece models.

When the CUDA toolkit emits a newer PTX ISA than the driver supports
(`CUDA_ERROR_UNSUPPORTED_PTX_VERSION`), use the wrapper in
`tools/nvcc-ptx-compat` (install it to `~/.local/bin/nvcc`) or install a
matching CUDA 13.0 toolkit. See `tools/nvcc-ptx-compat/README.md`.

```bash
NVCC_CCBIN=gcc-14 PATH=$PATH:/usr/local/cuda/bin cargo r --release --package translation-tool
NVCC_CCBIN=gcc-14 PATH=$PATH:/usr/local/cuda/bin cargo r --release --package spiegel-crawler --bin spiegel-crawler -- --userInfo <userInfo> --accessInfo <accessInfo> --userId <userId>
sudo dkms install --force nvidia/580.105.08 -k $(uname -r)
NVCC_CCBIN=gcc-14 NVCC_PREPEND_FLAGS = "-U_GNU_SOURCE -D_DEFAULT_SOURCE" PATH=$PATH:/usr/local/cuda-13.0/bin cargo r --release --package translation-tool
sudo ln -sf /usr/lib64/libcuda.so.1 /usr/lib/libcuda.so.1
sudo ln -sf /usr/lib64/libcuda.so.1 /usr/lib/libcuda.so
```

## adding voice

the tool will pick the items with translation == true and after generating the voice will set translation = voice. you still have to upload the voice data

### default voice

MISTRAL_API_KEY= cargo r --release --package voice-tool --bin voice-tool -- --prefix https://dek5ir2aw39om.cloudfront.net/ --voice-id a8b7df27-1b78-4411-8661-ade46c460b8e
### additional voice
MISTRAL_API_KEY= cargo r --release --package voice-tool --bin add_voice -- --voice-name merz --voice-id a8b7df27-1b78-4411-8661-ade46c460b8e
### upload the content
aws s3 sync generated s3://listentomeaha/generated

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


### version update

minimize the network traffic

used

```
implement concurrency/version check on the ui before fetching the data from the server. use an integer value what will be incrimentend whenever the articles data will change, this number will be persistend of the user preferences. when ever the article will be updated the persistent version will be increased store and the same version will be presistend on the article itself. alight this change for the crawler and translation tool.
```

planned

```
implement concurrency/version check on the ui before fetching the data from the server. use an integer value what will be incrimentend whenever the articles data will change, this number will be persistend of the user preferences. when ever the article will be updated the persistent version will be increased store and the same version will be presistend on the article itself. before persisting the change the application should check the latest version from the user prefesences and in case it's bigger that means that newer data on the server is available ask the user to refresh the data first and try to perist the change again. when fetching the data we check the version from the user preferences it this was not changed we already have the latest data and no article fetch is required. if newer data is available we only fetch articles with version number bigger than our current version update the articles by matching creation data that shound uniquely identify the artilce. the crawler should incrementent the number for the user preferences and use it when saving new articles. after done saving is update it for user preferences also. for the translation tool we build a map for next version number per user and while translating the article we set it on the article after we complete saving all we persist the new version numbers updated also on the user preferences.
```
