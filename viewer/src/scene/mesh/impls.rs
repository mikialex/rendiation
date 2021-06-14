use rendiation_renderable_mesh::mesh::IndexedMesh;

use super::{GPUMeshData, MeshCellGPU};

impl GPUMeshData for IndexedMesh {
  fn update(&self, gpu: &mut Option<MeshCellGPU>, device: wgpu::Device) {
    todo!()
  }
}
