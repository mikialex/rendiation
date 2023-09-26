
<p align="center">
  <img src="./asset/rrf.svg" alt="rrf logo" style="margin: auto">
</p>

# Rendiation Rendering Framework

RRF is a structured and comprehensive graphics framework for versatile interactive visualization requirements. 

The center of the framework consists of a production ready scene graph and a shader EDSL implementation. Several innovative ideas about rendering engine architecture design are explored. For example, the composability of the effects or optimization behaviors, the parallel reactive incremental systems, and the extensibility of the scene content representation.

Many handcrafted libraries of basic concepts in the graphics realm support the above center crates. Data structures, algorithms, and common operations in different graphics domains like mesh, texture, lighting, animation, and space partitions. Under these crates, there are foundation supports like mathematics, reactive primitives, generic containers, and platform graphics API abstractions(here we directly use and encapsulate wgpu). Leveraging these crates, users could build or even assemble and customize their own featured high-performance viewers, offline data assets pipelines, or any other highly demanded graphics-related tasks in a well-organized way with ease.

The RRF project incubates a demonstrative viewer for experimentation. This part is highly unfinished and under active development and design. Although the framework is largely aimed for 3d, we are also working on a new GUI solution with 2d rendering capabilities here.


## Development

Install the default rust toolchain and everything should works fine. RRF uses language features such as const generics and specialization, the nightly compiler is required. The cargo will automatically switch to the correct nightly version.

