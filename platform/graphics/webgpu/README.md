# WebGPU encapsulation layer

Contents and Responsibility:

* Provide clone-able and thread-safe wrapper for resource type.
* Record all related info with resource wrapper for convenience.
* Provide bindgroup cache and binding encapsulation, user could not care bindgroup management at all.
* Provide async functions for resource async read.
* Using stricter and polished api surface to reduce runtime exception.
* ~~Workaround annoying wgpu lifetime limitations, but also expose raw api for performance.~~
  * this issue is fixed upstream at wgpu version 22. however wgpu::RenderPass<'static> is ugly to use and some kind of the encapsulation still preferred.
* Bridge the shader and pipeline infrastructure to RRF shader api system.
* Potential performance optimization and inspection.
* Fix resource leaky behavior in web environment.
* Provider a range allocator for gpu buffer.
* Reexport common wgpu-types.
* Other shareable & reuseable components.
