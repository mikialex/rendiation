# Development Guide

## Backlog

The following things is the current project development direction.

### Basic correctness

- viewer huge memory leak
- emmisive has black shadow on default scene
- fix multiple taskgraph exist togther, ShaderFuture should impl shaderhashprovider
- fix texture pool grow
- mipmap and multi format support in texture pool system
- bindless mesh does not support optional uv and normal attributes
- optimize frame ctx "make_submit" call, use copy buffer to buffer to update.
- env background not support tonemapping
- alpha blending is not implemnt at all
- light uniform array not skip none exist light
- gizmo should not scaled by target scale
- use view space shading/lighting/postprocess compute to improve precision
- disable ssao when channel debug on
- fix channel debug in defer mode
- support material emissive larger than one
  - fix defer channel encode decode
  - fix gltf loader support
- fix db multi thead write lock access deadlock
- fix parallel compute test out of bound shader access
- fix scene gpu lighting is globally shared
- fix some mesh can not be picked in cpu picking (maybe related to u16 index format)
- fix missing blur pass in ssao
- fix ao should only shadowing diffuse lighting.
- ibl brdf lut should use higher precision lut
- fix outline shaking

### Indirect rendering

- fix mid count buffer missing indirect buffer usage flag
- implement basic indirect rendering capability
  - investigate current bindless texture performance issue(create binding group).

### Not yet integrated(tested) features

- widen line
- sky shading
- gpu driven occlusion culling
- on_demand_draw
- ssr(super naive)
- lod graph generation and rendering

### New features planed

- physical camera
- automatic exposure control
- oit
- good ssr
- visibility rendering
- cluster lighting optimization
  - dependency: storagebuffer light resources.
- ray tracing
  - reference path tracing renderer

### Infra and framework improvements planed

- ray tracing
  - new wavefront geometry backend by wgpu ray query api
  - improve the wavefront dispatch performance
    - let user manual control dispatch rounds
- support zero sized state in task graph

### Need help issue

- buffer combine with rtao shader breaks on Metal.

### Issues that require future dependency support

- correct hdr rendering, see <https://github.com/gfx-rs/wgpu/issues/2920>;

## Useful commands

most used testing and developing commands (write it down for convenience)

run main test viewer

```bash
cargo run --bin viewer
cargo run --release --bin viewer # run it in release mode
```

run given test when debugging. this is useful to fast relanch same test in terminal.

```bash
cargo t --package package_name test_name -- --nocapture
```

generate documents and open it in default browser (currently the project is extremely lack of documentation)

```bash
cargo doc --no-deps --open
```

 [the samply profiler](https://github.com/mstange/samply) is recommended to investigate cpu performance issue.  the most used command is:

```bash
cargo build --release --bin viewer
samply record ./target/release/viewer
```

For GPU debugging and profiling, the metal gpu capture is recommended to investigate gpu workload on macos. On the other platform that using Nvidia graphics card, the Nsight is recommended. If the webgpu backend switched to Dx12, the Pixi debugger is another good choice.

When using Xcode & Instrument to debug memory usage, your binary should manually signed or you will get "required kernel recording resources" error. see <https://github.com/rust-lang/rust/issues/107033>. Restart the profiler after signing.

## Coding style

The basic coding style is enforced by rustfmt. Some extra notes are:

- If the name of the struct or type contains multiple terminology nouns in sequence, for example "GPU" and "NDC" in "WebGPUNDC", use the "WebGPUxNDC" instead.
- Make sure the code looks visually comfortable, adjust the line break and insert empty row in pair with how logic and data flows. Rustfmt can not do that for you.

## Version control

- Avoid committing derived data, binary data (including bitmap images) into the repository,
  We're consider using a separate submodule repository for these assets.. except:
  - the file size is relatively small(less than 20kb), and
  - it's act as the fundamental support for some feature.
  
  For example the LUT texture used in rendering
