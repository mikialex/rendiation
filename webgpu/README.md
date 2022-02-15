## WebGPU encapsulation layer

Contents and Responsibility:

* much more strict and convenient api surface
* Provide builder like api for convenience;
* Workaround some annoy wgpu lifetime limitations for convenience, but also expose raw api for performance;
* Shareable & reuseable components
* Potential performance optimization and inspection