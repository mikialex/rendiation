use std::rc::Rc;

use bytemuck::Pod;
use gpu::util::DeviceExt;
use gpu::GPURenderPass;
use rendiation_webgpu as gpu;

use crate::group::*;
use crate::mesh::*;

pub struct MeshGPU {
  range_full: MeshGroup,
  vertex: Vec<Rc<gpu::Buffer>>,
  index: Option<(Rc<gpu::Buffer>, gpu::IndexFormat)>,
}

impl MeshGPU {
  pub fn get_range_full(&self) -> MeshGroup {
    self.range_full
  }

  pub fn setup_pass<'a>(&self, pass: &mut GPURenderPass<'a>) {
    self.vertex.iter().enumerate().for_each(|(i, gpu)| {
      pass.set_vertex_buffer_owned(i as u32, gpu);
    });
    if let Some((index, format)) = &self.index {
      pass.set_index_buffer_owned(index, *format);
    }
  }

  pub fn draw<'a>(&self, pass: &mut gpu::RenderPass<'a>, range: Option<MeshGroup>) {
    let range = range.unwrap_or(self.range_full);
    if self.index.is_some() {
      pass.draw_indexed(range.into(), 0, 0..1);
    } else {
      pass.draw(range.into(), 0..1);
    }
  }
}

/// The GPUMesh's cpu data source trait
pub trait GPUMeshData {
  fn update(&self, gpu: &mut Option<MeshGPU>, device: &gpu::Device);
  fn vertex_layout(&self) -> Vec<gpu::VertexBufferLayout>;
  fn get_group(&self, group: MeshDrawGroup) -> MeshGroup;
  fn topology(&self) -> gpu::PrimitiveTopology;
}

impl<I, V, T> GPUMeshData for GroupedMesh<IndexedMesh<I, V, T, Vec<V>>>
where
  V: Pod,
  T: PrimitiveTopologyMeta<V>,
  V: gpu::VertexBufferSourceType,
  I: gpu::IndexBufferSourceType,
  IndexedMesh<I, V, T, Vec<V>>: AbstractMesh,
{
  fn update(&self, gpu: &mut Option<MeshGPU>, device: &gpu::Device) {
    gpu.get_or_insert_with(|| self.mesh.create_gpu(device));
  }
  fn vertex_layout(&self) -> Vec<gpu::VertexBufferLayout> {
    vec![V::vertex_layout()]
  }

  fn get_group(&self, group: MeshDrawGroup) -> MeshGroup {
    self.get_group(group)
  }

  fn topology(&self) -> gpu::PrimitiveTopology {
    match T::ENUM {
      PrimitiveTopology::PointList => gpu::PrimitiveTopology::PointList,
      PrimitiveTopology::LineList => gpu::PrimitiveTopology::LineList,
      PrimitiveTopology::LineStrip => gpu::PrimitiveTopology::LineStrip,
      PrimitiveTopology::TriangleList => gpu::PrimitiveTopology::TriangleList,
      PrimitiveTopology::TriangleStrip => gpu::PrimitiveTopology::TriangleStrip,
    }
  }
}

impl<I, V, T> IndexedMesh<I, V, T, Vec<V>>
where
  V: Pod,
  T: PrimitiveTopologyMeta<V>,
  V: gpu::VertexBufferSourceType,
  I: gpu::IndexBufferSourceType,
  Self: AbstractMesh,
{
  pub fn create_gpu(&self, device: &gpu::Device) -> MeshGPU {
    let vertex = bytemuck::cast_slice(self.data.as_slice());
    let vertex = device.create_buffer_init(&gpu::util::BufferInitDescriptor {
      label: None,
      contents: vertex,
      usage: gpu::BufferUsages::VERTEX,
    });
    let vertex = vec![Rc::new(vertex)];

    let index = bytemuck::cast_slice(self.index.as_slice());
    let index = device.create_buffer_init(&gpu::util::BufferInitDescriptor {
      label: None,
      contents: index,
      usage: gpu::BufferUsages::INDEX,
    });
    let index = (Rc::new(index), I::FORMAT).into();

    let range_full = MeshGroup {
      start: 0,
      count: self.draw_count(),
    };

    MeshGPU {
      vertex,
      index,
      range_full,
    }
  }
}
