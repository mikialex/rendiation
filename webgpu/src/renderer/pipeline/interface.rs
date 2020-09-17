use rendiation_ral::{GeometryHandle, GeometryResourceInstance, ResourceManager};

use crate::WGPURenderer;

pub trait WGPUVertexProvider {
  fn get_buffer_layout_descriptor() -> wgpu::VertexBufferDescriptor<'static>;
}
pub trait WGPUGeometryProvider {
  fn get_geometry_vertex_state_descriptor() -> wgpu::VertexStateDescriptor<'static>;
  fn get_primitive_topology() -> wgpu::PrimitiveTopology;
  fn create_resource_instance(
    &self,
    renderer: &mut WGPURenderer,
    resource: &mut ResourceManager<WGPURenderer>,
  ) -> GeometryResourceInstance<WGPURenderer>;

  fn create_resource_instance_handle(
    &self,
    renderer: &mut WGPURenderer,
    resource: &mut ResourceManager<WGPURenderer>,
  ) -> GeometryHandle<WGPURenderer> {
    let instance = self.create_resource_instance(renderer, resource);
    resource.add_geometry(instance).index()
  }
}

pub trait WGPUBindGroupLayoutProvider: Sized + 'static {
  fn provide_layout(renderer: &WGPURenderer) -> wgpu::BindGroupLayout;
}
