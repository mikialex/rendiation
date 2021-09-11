use rendiation_renderable_mesh::{group::MeshDrawGroup, GPUMeshData, MeshGPU};
use rendiation_webgpu::GPU;
use std::marker::PhantomData;

use super::{Scene, TypedMeshHandle};

use rendiation_renderable_mesh::{group::GroupedMesh, mesh::IndexedMesh};
use rendiation_webgpu::VertexBufferSourceType;

pub trait GPUMeshLayoutSupport {
  type VertexInput;
}

impl<I, V, T> GPUMeshLayoutSupport for GroupedMesh<IndexedMesh<I, V, T, Vec<V>>>
where
  V: VertexBufferSourceType,
{
  type VertexInput = Vec<V>;
}

pub trait Mesh {
  fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, group: MeshDrawGroup);
  fn update(&mut self, gpu: &GPU);
  fn vertex_layout(&self) -> Vec<wgpu::VertexBufferLayout>;
  fn topology(&self) -> wgpu::PrimitiveTopology;
}

pub struct MeshCell<T> {
  data: T,
  gpu: Option<MeshGPU>,
}

impl<T: GPUMeshData> Mesh for MeshCell<T> {
  fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, group: MeshDrawGroup) {
    self
      .gpu
      .as_ref()
      .unwrap()
      .setup_pass(pass, self.data.get_group(group))
  }

  fn update(&mut self, gpu: &GPU) {
    self.data.update(&mut self.gpu, &gpu.device);
  }

  fn vertex_layout(&self) -> Vec<wgpu::VertexBufferLayout> {
    self.data.vertex_layout()
  }

  fn topology(&self) -> wgpu::PrimitiveTopology {
    self.data.topology()
  }
}

impl Scene {
  pub fn add_mesh<M>(&mut self, mesh: M) -> TypedMeshHandle<M>
  where
    M: GPUMeshData + 'static,
  {
    let handle = self.meshes.insert(Box::new(MeshCell {
      data: mesh,
      gpu: None,
    }));
    TypedMeshHandle {
      handle,
      ty: PhantomData,
    }
  }
}

/// the comprehensive data that provided by mesh and will affect graphic pipeline
pub struct MeshLayout {
  vertex: Vec<wgpu::VertexBufferLayout<'static>>,
  index: wgpu::IndexFormat,
  topology: wgpu::PrimitiveTopology,
}
