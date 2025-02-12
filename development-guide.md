# Development Guide

## Backlog

The following things is the current project development direction.

### Basic correctness

- use view space shading compute
- disable ssao when channel debug on
- fix channel debug in defer mode
- support material emissive larger than one
- fix db multi thead write lock access deadlock
- fix parallel compute test out of bound shader access
- fix scene gpu lighting is globally shared
- fix some mesh can not be picked (maybe related to u16 index format)
- fix viewer screenshot channel mismatch
- fix shader api serialization padding bug
- fix outline camera shaking
- fix missing blur pass in ssao
- ibl brdf lut should use higher precision lut

### Indirect rendering

- fix mid count buffer missing indirect buffer usage flag
- implement basic indirect rendering capability
  - investigate current bindless texture performance issue(create binding group).

### Not yet integrated(tested) features

- widen line
- sky shading
- gpu driven occlusion culling
- ltc lighting
- on_demand_draw

### New features planed

- gpu picking
- deferred rendering
- ray tracing
  - reference path tracing renderer

### Infra and framework improvements planed

- storage buffer virtual merge
- ray tracing
  - new wavefront geometry backend by wgpu ray query api
  - improve the wavefront dispatch performance
    - let user manual control dispatch rounds
- remove per frame large buffer recreation in parallel compute, fix the memory peak.

## Useful commands

most used testing and developing commands (write it down for convenience)

run main test viewer

```bash
cargo run --bin viewer
cargo run --release --bin viewer # run it in release mode
```

generate documents and open it in default browser

```bash
cargo doc --no-deps --open
```

If you're on macos or linux, [the samply profiler](https://github.com/mstange/samply) is recommended to investigate cpu performance issue.  the most used command is:

```bash
cargo build --release --bin viewer
samply record ./target/debug/viewer
```

For GPU debugging and profiling, the metal gpu capture is recommended to investigate gpu workload on macos. On the other platform that using Nvidia graphics card, the Nsight is recommended. If the webgpu backend switched to Dx12, the Pixi debugger is another good choice.

## Coding style

The coding style is enforced by rustfmt. Some extra notes are:

- If the name of the struct or type contains multiple terminology nouns in sequence, for example "GPU" and "NDC" in "WebGPUNDC", use the "WebGPUxNDC" instead.
- Make sure the code looks comfortable visually, adjust the line break and insert empty row in pair with how logic and data flows. Rustfmt can not do that for you.

## Version control

- Avoid committing derived data, binary data (including bitmap images) into the repository. We're consider using a separate submodule repository for these types of assets.
