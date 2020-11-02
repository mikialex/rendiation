use rendiation_ral::{GeometryHandle, GeometryProvider, GeometryResourceInstance, ResourceManager};

use crate::{WGPURenderer, WebGPU};

pub trait WGPUGeometryProvider {
  type Geometry: GeometryProvider<WebGPU>;
  fn get_geometry_vertex_state_descriptor() -> wgpu::VertexStateDescriptor<'static>;
  fn get_primitive_topology() -> wgpu::PrimitiveTopology;
  fn create_resource_instance(
    &self,
    renderer: &mut WGPURenderer,
    resource: &mut ResourceManager<WebGPU>,
  ) -> GeometryResourceInstance<WebGPU, Self::Geometry>;

  fn create_resource_instance_handle(
    &self,
    renderer: &mut WGPURenderer,
    resource: &mut ResourceManager<WebGPU>,
  ) -> GeometryHandle<WebGPU, Self::Geometry> {
    let instance = self.create_resource_instance(renderer, resource);
    resource.add_geometry(instance)
  }
}

pub trait WGPUBindGroupLayoutProvider: Sized + 'static {
  fn provide_layout(renderer: &WGPURenderer) -> wgpu::BindGroupLayout;
}
