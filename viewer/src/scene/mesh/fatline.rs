use crate::{Mesh, Scene};
use rendiation_renderable_mesh::{group::MeshDrawGroup, GPUMeshData, MeshGPU};
use rendiation_webgpu::GPU;

pub struct FatlineMeshCell<T> {
  data: T,
  gpu: Option<MeshGPU>,
}

impl<T> From<T> for FatlineMeshCell<T> {
  fn from(data: T) -> Self {
    Self { data, gpu: None }
  }
}

impl<T: GPUMeshData> Mesh for FatlineMeshCell<T> {
  fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, group: MeshDrawGroup) {
    self
      .gpu
      .as_ref()
      .unwrap()
      .setup_pass(pass, self.data.get_group(group).into())
  }

  fn update(&mut self, gpu: &GPU) {
    self.data.update(&mut self.gpu, &gpu.device);
  }

  fn vertex_layout(&self) -> Vec<wgpu::VertexBufferLayout> {
    self.data.vertex_layout()
  }

  fn topology(&self) -> wgpu::PrimitiveTopology {
    wgpu::PrimitiveTopology::TriangleList
  }
}

impl Scene {
  // pub fn add_mesh<M>(&mut self, mesh: M) -> TypedMeshHandle<M>
  // where
  //   M: GPUMeshData + 'static,
  // {
  //   let handle = self.meshes.insert(Box::new(MeshCell::from(mesh)));
  //   TypedMeshHandle {
  //     handle,
  //     ty: PhantomData,
  //   }
  // }
}
