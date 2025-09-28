# Rendiation Viewer

## wasm build

```bash
rustup target add wasm32-unknown-unknown
```

```bash
export RUSTFLAGS='--cfg getrandom_backend="wasm_js"'
cargo build --target wasm32-unknown-unknown -p viewer --release
```
