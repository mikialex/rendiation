# WebGPU encapsulation layer

Contents and Responsibility:

* Provide cloneable and thread-safe wrapper for resource type.
* Record all related info with resource wrapper for convenience.
* Provide bindgroup cache and binding encapsulation, user could not care bindgroup management at all.
* Provide async functions for resource async read.
* Using stricter and polished api surface to reduce runtime exception.
* Workaround annoying wgpu lifetime limitations, but also expose raw api for performance.
* Bridge the shader and pipeline infrastructure to RRF shader api system.
* Potential performance optimization and inspection.
* Other shareable & reuseable components.
