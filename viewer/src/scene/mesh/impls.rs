use bytemuck::Pod;
use rendiation_renderable_mesh::{
  group::{GroupedMesh, MeshGroup},
  mesh::{AbstractMesh, IndexedMesh, PrimitiveTopology, PrimitiveTopologyMeta},
};
use wgpu::util::DeviceExt;

use crate::scene::MeshDrawGroup;

use super::{GPUMeshData, IndexBufferSourceType, MeshGPU, VertexBufferSourceType};

impl<I, V, T> GPUMeshData for GroupedMesh<IndexedMesh<I, V, T, Vec<V>>>
where
  V: Pod,
  T: PrimitiveTopologyMeta<V>,
  Vec<V>: VertexBufferSourceType,
  I: IndexBufferSourceType,
  IndexedMesh<I, V, T, Vec<V>>: AbstractMesh,
{
  fn update(&self, gpu: &mut Option<MeshGPU>, device: &wgpu::Device) {
    gpu.get_or_insert_with(|| {
      let vertex = bytemuck::cast_slice(self.mesh.data.as_slice());
      let vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: vertex,
        usage: wgpu::BufferUsage::VERTEX,
      });
      let vertex = vec![vertex];

      let index = bytemuck::cast_slice(self.mesh.index.as_slice());
      let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: index,
        usage: wgpu::BufferUsage::INDEX,
      });
      let index = (index, I::FORMAT).into();

      MeshGPU { vertex, index }
    });
  }
  fn vertex_layout(&self) -> Vec<wgpu::VertexBufferLayout> {
    vec![Vec::<V>::vertex_layout()]
  }

  fn get_group(&self, group: MeshDrawGroup) -> MeshGroup {
    match group {
      MeshDrawGroup::Full => MeshGroup {
        start: 0,
        count: self.mesh.draw_count(),
      },
      MeshDrawGroup::SubMesh(i) => *self.groups.groups.get(i).unwrap(),
    }
  }

  fn topology(&self) -> wgpu::PrimitiveTopology {
    match T::ENUM {
      PrimitiveTopology::PointList => wgpu::PrimitiveTopology::PointList,
      PrimitiveTopology::LineList => wgpu::PrimitiveTopology::LineList,
      PrimitiveTopology::LineStrip => wgpu::PrimitiveTopology::LineStrip,
      PrimitiveTopology::TriangleList => wgpu::PrimitiveTopology::TriangleList,
      PrimitiveTopology::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
    }
  }
}

pub trait GPUMeshLayoutSupport {
  type VertexInput: VertexBufferSourceType;
}

impl<I, V, T> GPUMeshLayoutSupport for GroupedMesh<IndexedMesh<I, V, T, Vec<V>>>
where
  Vec<V>: VertexBufferSourceType,
{
  type VertexInput = Vec<V>;
}
