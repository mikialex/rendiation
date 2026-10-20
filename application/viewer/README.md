# Rendiation Viewer

## parametric model (.pnm)

A `.pnm` file (see `extension/parametric-data-normalization`) is loaded by drag and drop, by the
load button of the "Parametric Model" window (features menu) or by the `load-pnm [path]` command.
Set `RENDIATION_VIEWER_PNM=/path/to/model.pnm` (or `auto_load_pnm` in
`viewer_app_init_config.toml`) to load one at startup. The model is drawn by its own renderer with
a view dependent tessellation, not as a scene model (no material, lighting or picking). After
loading the camera is fitted to the model, press P to fit it again.

## wasm build

First install necessary dependencies

```bash
rustup target add wasm32-unknown-unknown
# this version must match wasm-bindgen version 
cargo install -f wasm-bindgen-cli --version 0.2.108
cargo install static-web-server
```

```bash
cargo xtask build-wasm

cargo xtask build-wasm --profiling # enable drawf debug and symbol for profiling

cargo xtask build-wasm --support-webgl # enable webgl support(but still prefers webgpu by default)

# assume in project root directory, in another terminal cx
static-web-server --config-file ./application/viewer-web/sws.toml
```

Then visit <http://127.0.0.1:6789/index.html>

deploy wasm viewer to github page:

```bash
cargo xtask build-deploy-wasm-github
```
