# GPU raytracing framework

## Long-term goal for design and implementation

- Align the API design with the Vulkan hardware raytracing API (referred to as vk-rtx).
- Develop a vk-rtx backend to fully leverage its performance advantages and capabilities.
- Create a compute shader backend using wavefront-tracing architecture.
  - Maximize cross-platform compatibility by enabling rendering on any target that supports compute shaders.
  - Investigate best practices, design decisions, and low-level trade-offs for vk-rtx.
  - Overcome limitations in the current vk-rtx specification.
  - Explore other ways to extend things, for example the alternative implementation of acceleration structures.
- Identify best engineering practices for GPU ray tracing:
  - Optimize the architecture advantages of our shader EDSL infrastructure.
  - Enhance the extensibility of effect features.
  - Establish best practices for resource management.
  - How to integrate into high-level frame and do hybrid rendering.
