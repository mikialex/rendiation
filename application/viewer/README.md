# Rendiation Viewer

## wasm build

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
```

```bash
export RUSTFLAGS='--cfg getrandom_backend="wasm_js"'
cargo build --target wasm32-unknown-unknown -p viewer --release
wasm-bindgen ./target/wasm32-unknown-unknown/release/viewer.wasm --target web --out-dir ./application/viewer-web/generated

http-server ./application/viewer-web
```
