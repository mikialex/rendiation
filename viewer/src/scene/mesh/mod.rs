use rendiation_renderable_mesh::{
  group::{MeshDrawGroup, MeshGroup},
  vertex::Vertex,
};
use rendiation_webgpu::GPU;
use std::marker::PhantomData;

use super::{Scene, TypedMeshHandle};

pub mod impls;
pub use impls::*;

pub trait Mesh {
  fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, group: MeshDrawGroup);
  fn update(&mut self, gpu: &GPU);
  fn vertex_layout(&self) -> Vec<wgpu::VertexBufferLayout>;
  fn topology(&self) -> wgpu::PrimitiveTopology;
}

pub trait GPUMeshData {
  fn update(&self, gpu: &mut Option<MeshGPU>, device: &wgpu::Device);
  fn vertex_layout(&self) -> Vec<wgpu::VertexBufferLayout>;
  fn get_group(&self, group: MeshDrawGroup) -> MeshGroup;
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

pub struct MeshGPU {
  vertex: Vec<wgpu::Buffer>,
  index: Option<(wgpu::Buffer, wgpu::IndexFormat)>,
}

impl MeshGPU {
  fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, range: MeshGroup) {
    self.vertex.iter().enumerate().for_each(|(i, gpu)| {
      pass.set_vertex_buffer(i as u32, gpu.slice(..));
    });
    if let Some((index, format)) = &self.index {
      pass.set_index_buffer(index.slice(..), *format);
      pass.draw_indexed(range.into(), 0, 0..1);
    } else {
      pass.draw(range.into(), 0..1);
    }
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

// use super::ValueID;

pub type MeshVertexLayout = Vec<wgpu::VertexBufferLayout<'static>>;

/// the comprehensive data that provided by mesh and will affect graphic pipeline
pub struct MeshLayout {
  vertex: MeshVertexLayout,
  index: wgpu::IndexFormat,
  topology: wgpu::PrimitiveTopology,
}

pub trait VertexBufferSourceType {
  fn vertex_layout() -> wgpu::VertexBufferLayout<'static>;
  fn get_shader_header() -> &'static str;
}

impl VertexBufferSourceType for Vec<Vertex> {
  fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
      array_stride: std::mem::size_of::<Vertex>() as u64,
      step_mode: wgpu::InputStepMode::Vertex,
      attributes: &[
        wgpu::VertexAttribute {
          format: wgpu::VertexFormat::Float32x3,
          offset: 0,
          shader_location: 0,
        },
        wgpu::VertexAttribute {
          format: wgpu::VertexFormat::Float32x3,
          offset: 4 * 3,
          shader_location: 1,
        },
        wgpu::VertexAttribute {
          format: wgpu::VertexFormat::Float32x2,
          offset: 4 * 3 + 4 * 3,
          shader_location: 2,
        },
      ],
    }
  }

  fn get_shader_header() -> &'static str {
    r#"
      [[location(0)]] position: vec3<f32>,
      [[location(1)]] normal: vec3<f32>,
      [[location(2)]] uv: vec2<f32>,
    "#
  }
}
