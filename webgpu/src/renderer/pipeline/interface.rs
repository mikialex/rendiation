use rendiation_ral::{GeometryHandle, GeometryProvider, GeometryResourceInstance, ResourceManager};

use crate::WGPURenderer;

pub trait WGPUVertexProvider {
  fn get_buffer_layout_descriptor() -> wgpu::VertexBufferDescriptor<'static>;
}
pub trait WGPUGeometryProvider {
  type Geometry: GeometryProvider<WGPURenderer>;
  fn get_geometry_vertex_state_descriptor() -> wgpu::VertexStateDescriptor<'static>;
  fn get_primitive_topology() -> wgpu::PrimitiveTopology;
  fn create_resource_instance(
    &self,
    renderer: &mut WGPURenderer,
    resource: &mut ResourceManager<WGPURenderer>,
  ) -> GeometryResourceInstance<WGPURenderer, Self::Geometry>;

  fn create_resource_instance_handle(
    &self,
    renderer: &mut WGPURenderer,
    resource: &mut ResourceManager<WGPURenderer>,
  ) -> GeometryHandle<WGPURenderer, Self::Geometry> {
    let instance = self.create_resource_instance(renderer, resource);
    resource.add_geometry(instance)
  }
}

pub trait WGPUBindGroupLayoutProvider: Sized + 'static {
  fn provide_layout(renderer: &WGPURenderer) -> wgpu::BindGroupLayout;
}
