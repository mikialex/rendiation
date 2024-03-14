# Rendiation Rendering Framework

RRF is a structured and comprehensive graphics sdk. "Rendiation" represents the concept of "rendering innovation".

The center of the framework consists of a gltf like scene api and a shader EDSL implementation. Several innovative rendering engine architecture design ideas are explored and applied. For example, the composability of complex effects or optimization behaviors, the parallel reactive incremental systems, and the extensibility of the scene content representation.

Many handcrafted libraries of basic concepts in the graphics realm support the above center crates. Data structures, algorithms, and common operations in different graphics domains like mesh, texture, lighting, animation, and space partitions. Under these crates, there are foundational supports like mathematics, reactive primitives, generic data containers, and platform graphics API abstractions(here we directly embrace and encapsulate wgpu).

By leveraging these crates, users can build, assemble and customize their own featured high-performance viewers, offline data assets pipelines, or any other highly demanded graphics-related tasks in a well-organized way with ease.

## Development

Install the default Rust toolchain, and everything should works fine.

RRF uses language features such as const generics and specialization, the nightly compiler is required. The Cargo will automatically switch to the correct nightly version.

## Project Structure

- math: foundational math libraries.
- platform: platform specifics & encapsulations, for example graphic API
- utility: general purpose data structure and algorithm and utils
- shader: language agnostic & dynamic programmable shader logic abstraction
- content: graphics related data structure and algorithm for domain related problem
- scene: general purpose 3d representation for data exchange, visualization & processing
- extension: extra implementations that using and extent the above ability in specific domain
- application: the user facing application for testing, prototyping and demonstrating
