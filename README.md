# Rendiation Rendering Framework

RRF is a rendering framework focused on performance and maintainability. "Rendiation" represents the concept of "rendering innovation".

For performance, RRF combines the innovative ideas of incremental computation and relational reactive programming, creating an incremental compute solution: the reactive query graph. Leveraging this, RRF reactively manages all derived data, internal state, caches, and GPU resources in the renderer at incremental cost. RRF also implements a state-of-the-art GPU-driven rendering pipeline, utilizing hardware advantages to approach zero driver overhead and minimize GPU workload.

Maintainability comes from simplicity and coherence, manifested as extensibility and composability. RRF uses an in-memory relational database to manage source-of-truth state, combined with the incremental system, reducing accidental complexity to a minimum. RRF carefully designs and composes axiom-level concepts, resulting in layered abstractions that correctly models modern rendering architecture and addresses its engineering challenges.

Another innovation is that RRF uses an EDSL shader system for all device logic. This architecture allows us to create highly dynamic, abstractive, and composable shader logic with a productive development experience. On this basis, we have features such as GPU parallel compute, GPU state machine, GPU reactive compute graph, GPU ray tracing, GPU virtual type system, and GPU error handling. These foundations enable a spectrum of capabilities, from low-level type system utilities to high-level application constructs, enriching shader programming to a new level.

## Project crates structure and scope

RRF is highly modular, layered, decoupled, and well-structured. Users can build, assemble, and customize their own high-performance viewers, offline data asset pipelines, or any other demanding graphics-related tasks. Here is the entry map of the project:

- math: foundational math libraries.
  - [algebra](./math/algebra/README.md): vectors, matrixes, transformations, projections, coordinate system definitions and abstract traits like vector space, inner product space.
  - geometry: primitives for example box, ray, plane, line segments, expressed by multi-dimensional structs and abstract geometry operations like intersection test, area measurement.
  - statistics: low discrepancy sequence generators, common distribution and useful mappings. an experimental sampling framework to express unbiased monte carlo sampling in a composable and extensible way.
- platform: platform specifics & encapsulations, for example graphic API
  - event_input: winit event utilities and common event process logic
  - graphics
    - [webgpu](./platform/graphics/webgpu/README.md): the wgpu encapsulation layer
    - webgpu-hook-utils: webgpu hooks extension support
    - webgpu-virtual-typed-combine-buffer: container utils that merge multiple buffer bindings into one to workaround binding count limitations on specific plaftform
- utility: general purpose data structure and algorithm and utils
  - abstract-graph: data structure independent graph algorithms
  - abstract-tree: data structure independent tree algorithms
  - database: an in-memory relational database and corresponding reactive watch system
  - heap-tools: useful tools to debug and monitoring memory leak related issue
  - query: LINQ like abstract composable data structure independent query operators. incremental stateful abstract query operators that embrace the core ideas of functional relational programming paradigm.
  - fast-hash-collection: just type exports, std-hash hash containers with a fast hash
  - storage: useful vector based util containers.
  - interning: type interning util
  - arena: a strong type generational arena
  - anymap: a simple type id keyed un-type value map container
  - widget: a simple widget definition with a view-state update cycle
- shader: language agnostic & dynamic programmable shader logic abstraction
  - [api](./shader/api/README.md): an EDSL like shader api. users use this to expressing their shading logic.
  - backends: the codegen or analysis backend for shader-api
    - naga: the wgpu-naga module builder backend
  - derive: macro implementation for shader-api
  - library: useful graphics shader logic collection
  - parallel-compute: express parallel computation in monad-like high level api
  - fast-down-sampling-2d: optimized mipmaps genenration using shared memory
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
  - view-override-model
  - wide-line
- application: the user facing application for testing, prototyping and demonstrating
  - rendiation-viewer
  - viewer-web: the wasm build of the viewer(note: incomplete and buggy)
    - online link: <https://mikialex.github.io/rendiation/viewer-web/index.html>

## Disclaimer and policy on LLM-assisted coding

All implementation before 2026 is written by hand. Starting from that, LLM-assisted coding tools may be used in:

- Code review (including reviewing legacy implementation)
- Test code generation
- Documentation
- Maintenance chore work
- Help implementation work and full foreign implementation porting(see below)
- Design discussion and research assistance, without producing any decisions

For implementation work: if the code is written with the help of LLM tools and it is a core feature (a feature that is guaranteed to be supported in the future and cannot be disabled in any way), then for that part:

- The implementation details must be understood line by line by the maintainer and fully and precisely express the maintainer's intent. The maintainer must be able to explain, modify, and extend every line without referring to the LLM session that produced it. The code must be continuously developable as if it were written entirely by the maintainer.
- The style and *taste* must match the rest of the project.

If the code is written largely with the help of LLM tools (even if it meets the above criteria), OR the code is written using LLM tools but cannot meet the above criteria (this is allowed if it is in a non-core part), such cases shall be recorded below for transparency. This record reflects the **current status** of each entry — once an entry is added, it is retained permanently even if the feature is later fully digested by the maintainer and no longer falls under this policy; in that case, an annotation may be added to indicate the change.

- Non-core system code written using LLM tools that cannot meet the above criteria:
  - `extension/dynamic-bvh` is an LLM port from parry's qbvh, using the rendiation math library.

## Development

Install the default Rust toolchain, and everything should works fine.RRF uses nightly language features, the Cargo will automatically switch to the correct nightly version.

more details see [development guide](./development-guide.md)
