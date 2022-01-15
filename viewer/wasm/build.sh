#!/usr/bin/env bash

export RUSTFLAGS=--cfg=web_sys_unstable_apis

cargo build --target wasm32-unknown-unknown $1 $2
wasm-bindgen --target web --out-dir ./wasm/build/ ../target/wasm32-unknown-unknown/debug/viewer.wasm
cd wasm 
static-server 
