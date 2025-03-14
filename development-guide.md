# Development Guide

## Backlog

The following things is the current project development direction.

### Correctness issues

important issue is in bold style.

- fix multiple taskgraph exist together, ShaderFuture should impl shaderhashprovider
- fix texture pool grow
- mipmap and multi format support in texture pool system
- bindless mesh does not support optional uv and normal attributes
- optimize frame ctx "make_submit" call, use copy buffer to buffer to update.
- alpha blending is not implement at all
- support per material custom state control
  - support double side config in gltf loader
  - fix gizmo plane move only one side is visible
- light uniform array and storage not skip none exist light
  - missing length info, breaks path tracing light sampling impl
- use view space shading/lighting/postprocess compute to improve precision
- disable ssao when channel debug on
- fix channel debug in defer mode
- support material emissive larger than one
  - fix defer channel encode decode
  - fix gltf loader support
- **fix db multi thead write lock access deadlock**
- fix parallel compute test out of bound shader access
- fix scene gpu lighting is globally shared
- fix some mesh can not be picked in cpu picking (maybe related to u16 index format)
- fix missing blur pass in ssao
- fix ao should only shadowing diffuse lighting.
- ibl brdf lut should use higher precision lut
- fix outline shaking
- integrate_brdf and ibl lighting shader code should reuse the std micro surface shading code
- indirect rendering
  - missing none index mesh, and line mesh draw support (camera helper not show)
  - lighting is not correct.

### Performance issues

- create binding group with any bindless texture is super slow. maybe upstream bug

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
  - support light and material MIS
  - support new high quality sampler

### Infra and framework improvements planed

- ray tracing
  - improve the wavefront dispatch performance
    - let user manual control dispatch rounds
- support zero sized state in task graph
- parallel compute support buffer reuse pool
- reactive query support parallel updates

### Need help issue

(empty)

### Upstream issue

- not reported or further investigate
  - naga metal backend has layout bug, (buffer combine with rtao shader breaks on Metal, workaound by adding manual padding in struct end).
- known but not fixed yet
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

## Tracy profiler

[tracy](https://github.com/wolfpld/tracy) is a useful tool to investigate memory and performance issues. Tracy has already integrated into viewer application. It is disabled by default behind the `tracy` feature flag because it will using and buffering a lot of memory when enabled. The current integration should use tracy 0.11.1 client to connect.

```bash
cargo run --bin viewer --features "tracy" # run viewer enable tracy
cargo run --bin viewer --features "tracy-heap-debug" # run viewer enable tracy and tracy-heap-debug
```

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
