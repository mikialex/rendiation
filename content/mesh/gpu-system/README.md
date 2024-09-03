# Bindless mesh gpu system

This crate helps you manage the "bindless mesh", which is the mesh group that could draw by dispatching a single multi-draw command buffer. To do this, the mesh buffers(at least index buffer if you using the bindless storage buffer instead of vertex buffer) should be allocated in one single large buffer, maintain their range info, and be able to generate the draw command both from the host and device side.
