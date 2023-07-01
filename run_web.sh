#!/usr/bin/env bash
rustup target add wasm32-unknown-unknown
cargo install basic-http-server
cargo build --target wasm32-unknown-unknown --release
rm -r web_gen
wasm-bindgen --out-dir ./web_gen/ --target web ./target/wasm32-unknown-unknown/release/caticorn.wasm
cp wasm/* web_gen/
cp -r assets web_gen/
basic-http-server web_gen
