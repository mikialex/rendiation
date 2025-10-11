# Rendiation Viewer

## wasm build

First install necessary dependencies

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
cargo install static-web-server
```

```bash
cargo xtask build-wasm

# assume in project root directory, in another terminal cx
static-web-server --config-file ./application/viewer-web/sws.toml
```

Then visit <http://127.0.0.1:6789/index.html>
