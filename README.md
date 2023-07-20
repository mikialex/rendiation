
<p align="center">
  <img src="./asset/rrf.svg" alt="rrf logo" style="margin: auto">
</p>

# Rendiation Rendering Framework

RRF is a well-designed comprehensive graphics framework to solve versatile interactive visualization requirements. 

The center of the framework consists of a scene graph and a shader graph implementation. Several innovative ideas for the rendering engine architecture are explored. For example, the composability of the effects or optimization behaviors, the parallel reactive incremental systems, and the extensibility of the content representation.  The core features are relatively mature and ready to be used in a production environment.

Many handcrafted libraries of basic concepts in graphics support the center crates. The data structures, algorithms, and common operations in different domains like mesh, texture, lighting, animation, and space partitions. Under these crates, there are foundation supports like mathematics, reactive primitives, generic containers, and platform graphics API abstractions. Leveraging these crates, users could build or even assemble their own featured high-performance viewers, offline data assets pipelines, or any other highly demanded graphics-related tasks in a well-organized way with ease.

The RRF project incubates a demonstrative viewer for experimentation. This part is highly unfinished and under active development. We are also working on a new GUI solution here.

RRF uses language features such as const generics and specialization, the nightly compiler is required.