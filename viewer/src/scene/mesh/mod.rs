use std::marker::PhantomData;

use bytemuck::Pod;
use rendiation_renderable_mesh::{group::MeshGroup, vertex::Vertex};

use crate::Renderer;

use super::{MeshDrawGroup, MeshHandle, Scene};

pub mod impls;
pub use impls::*;

pub struct TypedMeshHandle<T> {
  handle: MeshHandle,
  ty: PhantomData<T>,
}

impl<T> Into<MeshHandle> for TypedMeshHandle<T> {
  fn into(self) -> MeshHandle {
    self.handle
  }
}

pub trait Mesh {
  fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, group: MeshDrawGroup);
  fn update(&mut self, renderer: &mut Renderer);
  fn vertex_layout(&self) -> Vec<wgpu::VertexBufferLayout>;
}

pub trait GPUMeshData {
  fn update(&self, gpu: &mut Option<MeshGPU>, device: &wgpu::Device);
  fn vertex_layout(&self) -> Vec<wgpu::VertexBufferLayout>;
  fn get_group(&self, group: MeshDrawGroup) -> MeshGroup;
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

  fn update(&mut self, renderer: &mut Renderer) {
    self.data.update(&mut self.gpu, &renderer.device);
  }

  fn vertex_layout(&self) -> Vec<wgpu::VertexBufferLayout> {
    self.data.vertex_layout()
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

pub trait VertexBufferSourceType: Pod {
  fn vertex_layout() -> wgpu::VertexBufferLayout<'static>;
  fn get_shader_header() -> &'static str;
}

impl VertexBufferSourceType for Vertex {
  fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
      array_stride: std::mem::size_of::<Self>() as u64,
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

pub trait IndexBufferSourceType: Pod {
  const FORMAT: wgpu::IndexFormat;
}

impl IndexBufferSourceType for u32 {
  const FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint32;
}

impl IndexBufferSourceType for u16 {
  const FORMAT: wgpu::IndexFormat = wgpu::IndexFormat::Uint16;
}
