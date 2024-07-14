from amazonlinux:latest

run yum update && yum install gcc openssl-devel npm tar which gzip -y
RUN npm install -g sass
user 1000:1000
env HOME=/tmp
run curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
env PATH="$HOME/.cargo/bin:$PATH"
RUN curl --proto '=https' --tlsv1.2 -LsSf https://github.com/leptos-rs/cargo-leptos/releases/latest/download/cargo-leptos-installer.sh | sh
RUN rustup target add wasm32-unknown-unknown
workdir /app
cmd cargo leptos build --release -vv ; npx tailwindcss -i ./input.css -o ./public/typing.css
