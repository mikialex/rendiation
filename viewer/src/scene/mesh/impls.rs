use rendiation_renderable_mesh::mesh::IndexedMesh;

use super::{GPUMeshData, MeshCellGPU};

impl<I, V, T, U> GPUMeshData for IndexedMesh<I, V, T, U> {
  fn update(&self, gpu: &mut Option<MeshCellGPU>, device: wgpu::Device) {
    todo!()
  }
}
