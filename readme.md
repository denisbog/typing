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

## article pipeline

Articles are downloaded and translated outside this repository:

* `~/llm/crawler` downloads each SPIEGEL article — text, images and the
  narration audio — into one folder per article: `article.json`, `images/`,
  `audio/`.
* `~/llm/transcribe` force-aligns the article text to the audio and translates
  it, writing `transcribe/transcript.json` (sentence timings),
  `transcribe/translation.json` (the English sentences) and
  `transcribe/transcription.json` (word-level timings).

The media is uploaded to the cloud store (S3/CloudFront) by hand. The
`article-import` tool reads the article folders and creates the DynamoDB record
the web app serves, aligning paragraphs and headings with the translated
sentences and their audio timings:

```bash
cargo run --release --package article-import -- \
    --user-id <uuid> \
    --prefix https://<distribution>.cloudfront.net/articles \
    --root ~/llm/crawler/articles-ihre-artikel-new
```

`--dir <article-folder>` imports a single article (repeatable), `--dry-run`
prints what would be imported without writing, and `--dump-json` prints the
record. One run bumps the user's library version once and stamps every article
with it, so the app's incremental sync sees a single revision.


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
