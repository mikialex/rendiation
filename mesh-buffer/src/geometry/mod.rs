pub mod bvh;
pub mod container;
pub mod conversion;
pub mod intersection;
pub mod primitive;

use bytemuck::cast_slice;
pub use container::*;
pub use primitive::*;

pub use bvh::*;
pub use intersection::*;
use rendiation_math_entity::Positioned3D;
use rendiation_ral::{
  GeometryDescriptorProvider, GeometryProvider, GeometryResourceCreator, GeometryResourceInstance,
  GeometryResourceInstanceCreator, IndexFormat, ResourceManager, VertexBufferDescriptorProvider,
  VertexStateDescriptor, VertexStateDescriptorProvider, RAL,
};

impl<'a, V, T, U, R> GeometryResourceCreator<R> for IndexedGeometry<u16, V, T, U>
where
  V: Positioned3D + GeometryProvider,
  T: PrimitiveTopology<V>,
  U: RALGeometryDataContainer<V, R> + 'static,
  R: RAL,
{
  type Instance = GeometryResourceInstance<R, V>;

  fn create(
    &self,
    resources: &mut ResourceManager<R>,
    renderer: &mut R::Renderer,
  ) -> Self::Instance {
    let mut instance = GeometryResourceInstance::new();
    let index_buffer = R::create_index_buffer(renderer, cast_slice(&self.index));
    instance.index_buffer = Some(resources.add_index_buffer(index_buffer).index());

    self.data.create_gpu(resources, renderer, &mut instance);
    instance.draw_range = 0..self.get_full_count();
    instance
  }
}

impl<V, T, U, R> GeometryResourceInstanceCreator<R, V> for IndexedGeometry<u16, V, T, U>
where
  V: Positioned3D + GeometryProvider,
  T: PrimitiveTopology<V>,
  U: RALGeometryDataContainer<V, R> + 'static,
  R: RAL,
{
}

impl<'a, V, T, U> VertexStateDescriptorProvider for IndexedGeometry<u16, V, T, U>
where
  V: Positioned3D + VertexBufferDescriptorProvider,
  T: PrimitiveTopology<V>,
  U: GeometryDataContainer<V>,
{
  fn create_descriptor() -> VertexStateDescriptor<'static> {
    VertexStateDescriptor {
      index_format: IndexFormat::Uint16,
      vertex_buffers: &[V::DESCRIPTOR],
    }
  }
}

impl<'a, V, T, U> GeometryDescriptorProvider for IndexedGeometry<V, T, U>
where
  V: Positioned3D + VertexBufferDescriptorProvider,
  T: PrimitiveTopology<V>,
  U: GeometryDataContainer<V>,
{
  fn get_primitive_topology() -> rendiation_ral::PrimitiveTopology {
    T::ENUM
  }
}
