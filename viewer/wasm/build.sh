#!/usr/bin/env bash


export RUSTFLAGS=--cfg=web_sys_unstable_apis  
# cargo build --target wasm32-unknown-unknown --release 
# wasm-bindgen --target web --out-dir ./wasm/build/ ../target/wasm32-unknown-unknown/release/viewer.wasm

cargo build --target wasm32-unknown-unknown 
wasm-bindgen --target web --out-dir ./wasm/build/ ../target/wasm32-unknown-unknown/debug/viewer.wasm
cd wasm 
static-server 
