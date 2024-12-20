# Rendiation Rendering Framework

RRF is a structured and comprehensive graphics framework. "Rendiation" represents the concept of "rendering innovation".

The core of the framework consists of a gltf like scene api and a shader EDSL implementation. Many innovative rendering engine architecture design ideas are explored and implemented. For example, the composability of complex effects or optimization behaviors, the parallel reactive incremental systems, and the extensibility of the scene content representation.

Many handcrafted libraries of basic concepts in the graphics realm support the above center crates. Data structures, algorithms, and common operations in different graphics domains like mesh, texture, lighting, animation, and space partitions. Under these crates, there are foundational supports like mathematics, reactive primitives, generic data containers, and platform graphics API abstractions(here we directly embrace and encapsulate wgpu).

By leveraging these crates, users can build, assemble and customize their own featured high-performance viewers, offline data assets pipelines, or any other highly demanded graphics-related tasks in a well-organized way with ease.

## Project crates structure and scope

- math: foundational math libraries.
  - [algebra](./math/algebra/README.md): vectors, matrixes, transformations, projections, coordinate system definitions and abstract traits like vector space, inner product space.
  - geometry: primitives for example box, ray, plane, line segments, expressed by multi-dimensional structs and abstract geometry operations like intersection test, area measurement.
  - statistics: low discrepancy sequence generators, common distribution and useful mappings. an experimental sampling framework to express unbiased monte carlo sampling in a composable and extensible way.
- platform: platform specifics & encapsulations, for example graphic API
  - event_input: winit event utilities and common event process logic
  - graphics
    - [webgpu](./platform/graphics/webgpu/README.md): the wgpu encapsulation layer
    - webgpu-reactive-utils: reactive webgpu resource management containers base on our reactive infrastructure
- utility: general purpose data structure and algorithm and utils
  - abstract-graph: data structure independent graph algorithms
  - abstract-tree: data structure independent tree algorithms
  - database: an in-memory relational database and corresponding reactive watch system
  - heap-tools: useful tools to debug and monitoring memory leak related issue
  - query: LINQ like abstract composable data data structure independent query operators.
  - reactive: reexport the following reactive utils:
    - reactive-query: incremental reactive stateful abstraction query operators that embrace the core ideas of functional relational programming paradigm.
    - reactive-stream: signal/stream-like reactive operators.
    - reactive-derive: macro implementation for reactive utils
  - fast-hash-collection: just type exports, std-hash hash containers with a fast hash
  - storage: useful vector based util containers.
  - interning: type interning util
  - dyn-downcast: a util to impl runtime virtual table
  - arena: a strong type generational arena
  - widget: an simple widget definition with a view-state update cycle
- shader: language agnostic & dynamic programmable shader logic abstraction
  - [api](./shader/api/README.md): an EDSL like shader api. users use this to expressing their shading logic.
  - backends: the codegen or analysis backend for shader-api
    - naga: the wgpu-naga module builder backend
  - derive: macro implementation for shader-api
  - library: useful graphics shader logic collection
  - parallel-compute: express parallel computation in monad-like high level api
  - task-graph: a on-device task-graph runtime to support massively stateful computation in gpu.
  - [ray-tracing](./shader/ray-tracing/README.md): a monad-like high level api for gpu ray tracing with the same capabilities as the native hardware rtx. and a wavefront tracing executor based on task-graph.
- content: graphics related data structure and algorithm for domain related problem
  - texture
    - core
    - gpu-base
    - gpu-process: image based gpu texture processing tools, for example SSAO, TAA
    - [gpu-system](./content/texture/gpu-system/README.md): texture gpu abstractions and advance implementations for example bindless texture, texture pool.
    - loader: file loader support
    - [packer](./content/texture/packer/README.md): rectangle packing algorithms
    - types
  - mesh
    - core: abstractions and data containers
    - generator: generate mesh by define and compose parametric geometry surface
    - lod-graph: Unreal Nanite-like mesh LOD graph
    - [segmentation](./content/mesh/segmentation/README.md)
    - [simplification](./content/mesh/simplification/README.md)
  - lighting
    - core
    - gpu-system: gpu lighting compute utils
    - [ibl](./content/lighting/ibl/README.md): IBL environmental lighting
    - [ltc](./content/lighting/ltc/README.md): LTC area lighting and lut map generator
    - punctual
    - transport
  - space: utils for space indexer algorithm for example BVH BSP.
  - color: utils for colors
  - animation: utils for animations
- scene: general purpose 3d representation for data exchange, visualization & processing
  - core: scene object model definitions based on relational data model
  - geometry-query: ray-picking utils for scene
  - io: file io supports for scene
  - rendering
    - gpu-base: shareable scene rendering abstraction and implementations for scene
    - gpu-gles: a "GLES300" style(mostly used in compatible mode) rendering implementation for scene
    - gpu-indirect: an indirect style(gpu driven mode) rendering implementation for scene
    - gpu-ray-tracing: an experimental offline gpu raytracing rendering implementation for scene
- extension: extra implementations that leverage and extent the above abilities in specific domain
  - barycentric-solid-line
  - controller
  - gizmo
  - gui-3d
  - infinity-primitive
  - scene-debug-helper
  - state-override
  - view-override-model
  - wide-line
- application: the user facing application for testing, prototyping and demonstrating
  - viewer

## Development

Install the default Rust toolchain, and everything should works fine.RRF uses nightly language features, the Cargo will automatically switch to the correct nightly version.

more details see [development guide](./development-guide.md)
