use std::marker::PhantomData;

use crate::*;
use rendiation_algebra::*;
use rendiation_webgpu::*;

use rendiation_renderable_mesh::{
  group::MeshDrawGroup, mesh::NoneIndexedMesh, vertex::Vertex, GPUMeshData, MeshGPU,
};

pub type FatlineData = NoneIndexedMesh;

pub struct FatlineMeshCell {
  data: FatlineData,
  gpu: Option<MeshGPU>,
}

impl From<FatlineData> for FatlineMeshCell {
  fn from(data: FatlineData) -> Self {
    Self { data, gpu: None }
  }
}

impl Mesh for FatlineMeshCell {
  fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, group: MeshDrawGroup) {
    // self
    //   .gpu
    //   .as_ref()
    //   .unwrap()
    //   .setup_pass(pass, self.data.get_group(group).into())
  }

  fn update(&mut self, gpu: &GPU) {
    // self.data.update(&mut self.gpu, &gpu.device);
  }

  fn vertex_layout(&self) -> Vec<wgpu::VertexBufferLayout> {
    vec![FatLineVertex::vertex_layout(), Vertex::vertex_layout()]
  }

  fn topology(&self) -> wgpu::PrimitiveTopology {
    wgpu::PrimitiveTopology::TriangleList
  }
}

pub type FatlineMeshHandle = TypedHandle<FatlineMeshCell, MeshHandle>;

impl Scene {
  pub fn add_fatline_mesh<M>(&mut self, mesh: FatlineData) -> FatlineMeshHandle
  where
    M: GPUMeshData + 'static,
  {
    let handle = self.meshes.insert(Box::new(FatlineMeshCell::from(mesh)));
    TypedMeshHandle {
      handle,
      ty: PhantomData,
    }
  }
}

pub struct FatLineVertex {
  start: Vec3<f32>,
  end: Vec3<f32>,
  color: Vec3<f32>,
}

impl VertexBufferSourceType for FatLineVertex {
  fn vertex_layout() -> VertexBufferLayout<'static> {
    VertexBufferLayout {
      array_stride: std::mem::size_of::<Self>() as u64,
      step_mode: VertexStepMode::Instance,
      attributes: &[
        VertexAttribute {
          format: VertexFormat::Float32x3,
          offset: 0,
          shader_location: 0,
        },
        VertexAttribute {
          format: VertexFormat::Float32x3,
          offset: 4 * 3,
          shader_location: 1,
        },
        VertexAttribute {
          format: VertexFormat::Float32x3,
          offset: 4 * 3 + 4 * 3,
          shader_location: 2,
        },
      ],
    }
  }

  fn get_shader_header() -> &'static str {
    r#"
      [[location(1)]] fatline_start: vec3<f32>,
      [[location(2)]] fatline_end: vec3<f32>,
      [[location(3)]] fatline_color: vec3<f32>,
    "#
  }
}
