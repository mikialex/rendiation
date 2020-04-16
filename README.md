# Rendiation Rendering Framework & mikialex 's GFX crates

## Rendiation

This repo is in very early stage.

Some direction:

* In high level abstraction, port the architecture implementation of [artgl](https://github.com/mikialex/artgl).
* In low level abstraction, wrap wgpu-rs and provide lots of convenience api such as builder patten, marcos.., as well as composable render primitives.

Use WebGPU as backend support. Support multiple native platform and web.

## Other graphics crates also exist here

RiverMesh for mesh processing experiments.

Rendium for UI.

Noize for noise pattern generation.

RainRay for ray tracing

RineCraft for minecraft like game dev experiments.

SpaceIndexer for space acceleration data structure algorithms.
...

Include them in one mono repo is to make code reuse easier and clean.
