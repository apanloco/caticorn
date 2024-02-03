#!/usr/bin/env bash
set -e
set -x
env
if [ -n "${RENDER}" ]; then
  wget https://github.com/WebAssembly/binaryen/releases/download/version_116/binaryen-version_116-x86_64-linux.tar.gz
  tar zxvf *binaryen*.tar.gz
  export PATH=$PATH:$(pwd)/binaryen-version_116/bin  
  wasm-opt --version
fi
cat /etc/*release*
uname -a
rustc --version
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
rm -rf build
cargo install wasm-bindgen-cli --version 0.2.87
wasm-bindgen --out-dir ./build/ --target web ./target/wasm32-unknown-unknown/release/caticorn.wasm
ls -l build
wasm-opt build/caticorn_bg.wasm -Oz -o build/caticorn_bg.wasm
ls -l build
cp web/* build/
cp -r assets build/
