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

cargo xtask build-wasm --profiling # enable drawf debug and symbol for profiling

cargo xtask build-wasm --webgl # enable webgl support and forced using webgl

# assume in project root directory, in another terminal cx
static-web-server --config-file ./application/viewer-web/sws.toml
```

Then visit <http://127.0.0.1:6789/index.html>
