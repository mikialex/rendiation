# Rendiation Viewer

## wasm build

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
```

```bash
cargo xtask build-wasm

http-server ./application/viewer-web
```
