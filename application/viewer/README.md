# Rendiation Viewer

## wasm build

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
cargo install static-web-server
```

```bash
cargo xtask build-wasm

# assume in project root directory
static-web-server --config-file ./application/viewer-web/sws.toml
```
