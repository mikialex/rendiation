# Development Guide

## Backlog

The following things is the current project development direction.

### Improve gpu ray-tracing infrastructure and feature set

- improve the wavefront dispatch performance
  - let user manual control dispatch rounds
- improve the wavefront memory management
  - improve the parallel compute infra, remove per frame large buffer recreation, fix the memory peak.
- implement a reference path tracing renderer
- implement new wavefront geometry backend by wgpu ray query api
- investigate how to support metal and web (greatly reduce storage binding count)

### Indirect rendering

- implement basic indirect rendering capability
  - investigate current bindless texture performance issue(create binding group).

### Basic correctness

- use view space shading compute
- disable ssao when channel debug on
- fix normal matrix
- fix scene gpu lighting is globally shared
- fix some mesh can not be picked (maybe related to u16 index format)
- fix viewer screenshot channel mismatch
- fix shader api serialization padding bug

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
samply record cargo r --release viewer
```

For GPU debugging and profiling, the metal gpu capture is recommended to investigate gpu workload on macos. On the other platform that using Nvidia graphics card, the Nsight is recommended. If the webgpu backend switched to Dx12, the Pixi debugger is another good choice.

## Coding style

The coding style is enforced by rustfmt. Some extra notes are:

- If the name of the struct or type contains multiple terminology nouns in sequence, for example "GPU" and "NDC" in "WebGPUNDC", use the "WebGPUxNDC" instead.
- Make sure the code looks comfortable visually, adjust the line break and insert empty row in pair with how logic and data flows. Rustfmt can not do that for you.

## Version control

- Avoid committing derived data, binary data (including bitmap images) into the repository. We're consider using a separate submodule repository for these types of assets.
