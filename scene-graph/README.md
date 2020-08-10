# SceneGraph

Provide a 3d scene hierarchy structure for general usage. Support any backend that impl RALBackend(in rendiation_ral). Backend resource type and rendering specialized impl will be inject by generics trait implementation.

## Web Platform support

The wasm-bindgen not support generics thing on api, so under the wasm folder we wrap the scene to avoid generic expose. Scene api are all wrapped under `usize` handle instead of `Handle<T>`, Every wasm js communication through usize handle. It's obviously that the handle like api is awkward to use, to provide a ergonomic js/ts api as many popular frontend 3d frameworks like threejs, we create a customized web side api warp as the client of wasm part. These impls as well as demo code settled in ./web folders.

### how to build

wasm side: Install wasm-pack tool-chain, `cd <project-root>/scene-graph && wasm-pack build`

web side: Install node modules by yarn, run webpack dev server