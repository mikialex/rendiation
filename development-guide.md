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

- fix scene gpu lighting is globally shared
- fix some mesh can not be picked (maybe related to u16 index format)

## Misc

most used testing and developing commands (write it down for convenience)

run main test viewer

```bash
cargo run viewer
cargo run --release viewer # run it in release mode
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
