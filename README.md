
<p align="center">
  <img src="./asset/rrf.svg" alt="rrf logo" style="margin: auto">
</p>

# Rendiation Rendering Framework

RRF is a group of crates that can be composed to impl graphics project. Build your own renderer, realtime viewer or offline tracer, game engine, physics engine, graphics data processing, generative content creation..

This repo is in very early stage and very unstable. most of crates highly unfinished.

RRF use language features such as const generics and specialization, nightly compiler required.

## Crates

### Primitives

* Math for linear algebra primitives. Vec. Mat..
* MathEntity for geometric primitives. Box. Sphere..
* RenderEntity for graphics primitives. Camera. Controller..

### Rendering

* RendiationWebGPU for WebGPU.
* RendiationWebGL for WebGL2.

### Framework

* [SceneGraph](./scene-graph/README.md) for graphics API agnostic 3D scene description and rendering;
* [ShaderGraph](./shader-graph/README.md) for Shading Abstraction Layer;
* RenderGraph for graphics API agnostic multi-pass dependency resolve and composition;
* Rendium for UI system.

### Library

* MeshBuffer for geometry mesh creating conversion utils.
* SpaceIndexer for space acceleration data structure algorithms.
* RiverMesh for mesh processing experiments. (mesh edit)
* Noize for noise pattern generation.

### Application incubation

* [Voxland](./voxland/README.md) for testing minecraft like game.
* RainRay for yet another ray tracer.
