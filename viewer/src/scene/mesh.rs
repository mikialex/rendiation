use bytemuck::Pod;
use rendiation_renderable_mesh::vertex::Vertex;
use wgpu::util::DeviceExt;

use crate::Renderer;
use std::any::Any;

use super::ValueID;

/// the comprehensive data that provided by mesh and will affect graphic pipeline
pub struct MeshLayout {
  vertex: MeshVertexLayout,
  index: wgpu::IndexFormat,
  topology: wgpu::PrimitiveTopology,
}

pub struct SceneMesh {
  layout: ValueID<MeshLayout>,
  vertex: Vec<VertexBuffer>,
  index: Option<IndexBuffer>,
}

impl SceneMesh {
  pub fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
    self.index.as_ref().map(|index| index.setup_pass(pass));
    self
      .vertex
      .iter()
      .enumerate()
      .for_each(|(i, vertex)| vertex.setup_pass(pass, i as u32))
  }
}

pub trait VertexBufferSourceType: Pod {
  fn get_layout() -> wgpu::VertexBufferLayout<'static>;
  fn get_shader_header() -> &'static str;
}

impl VertexBufferSourceType for Vertex {
  fn get_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
      array_stride: std::mem::size_of::<Self> as wgpu::BufferAddress,
      step_mode: wgpu::InputStepMode::Vertex,
      attributes: &[
        wgpu::VertexAttribute {
          format: wgpu::VertexFormat::Float32x4,
          offset: 0,
          shader_location: 0,
        },
        wgpu::VertexAttribute {
          format: wgpu::VertexFormat::Float32x2,
          offset: 4 * 4,
          shader_location: 1,
        },
      ],
    }
  }

  fn get_shader_header() -> &'static str {
    r#"
      [[location(0)]]
      var<in> in_position: vec4<f32>;
      [[location(1)]]
      var<in> in_tex_coord_vs: vec2<f32>;
      "#
  }
}

pub type MeshVertexLayout = Vec<wgpu::VertexBufferLayout<'static>>;

pub trait VertexBufferSource: 'static {
  fn as_any(&self) -> &dyn Any;
  fn as_bytes(&self) -> &[u8];
  fn get_layout(&self) -> MeshVertexLayout;
  fn get_shader_header(&self) -> &'static str;
}

impl<T: VertexBufferSourceType> VertexBufferSource for Vec<T> {
  fn as_any(&self) -> &dyn Any {
    self
  }
  fn as_bytes(&self) -> &[u8] {
    bytemuck::cast_slice(self.as_slice())
  }
  fn get_layout(&self) -> MeshVertexLayout {
    vec![T::get_layout()]
  }
  fn get_shader_header(&self) -> &'static str {
    T::get_shader_header()
  }
}

pub struct VertexBuffer {
  data: Box<dyn VertexBufferSource>,
  gpu: Option<wgpu::Buffer>,
}

impl VertexBuffer {
  pub fn new(data: impl VertexBufferSource) -> Self {
    let data = Box::new(data);
    Self { data, gpu: None }
  }

  pub fn update(&mut self, renderer: &mut Renderer) {
    let data = &self.data;
    self.gpu.get_or_insert_with(|| {
      let device = &renderer.device;
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: data.as_bytes(),
        usage: wgpu::BufferUsage::VERTEX,
      })
    });
  }

  pub fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, slot: u32) {
    let gpu = self.gpu.as_ref().unwrap();
    pass.set_vertex_buffer(slot, gpu.slice(..));
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

pub trait IndexBufferSource: 'static {
  fn as_any(&self) -> &dyn Any;
  fn as_bytes(&self) -> &[u8];
  fn index_format(&self) -> wgpu::IndexFormat;
}

impl<T: IndexBufferSourceType> IndexBufferSource for Vec<T> {
  fn as_any(&self) -> &dyn Any {
    self
  }
  fn as_bytes(&self) -> &[u8] {
    bytemuck::cast_slice(self.as_slice())
  }
  fn index_format(&self) -> wgpu::IndexFormat {
    T::FORMAT
  }
}

pub struct IndexBuffer {
  data: Box<dyn IndexBufferSource>,
  gpu: Option<wgpu::Buffer>,
}

impl IndexBuffer {
  pub fn new(data: impl IndexBufferSource) -> Self {
    let data = Box::new(data);
    Self { data, gpu: None }
  }

  pub fn update(&mut self, renderer: &mut Renderer) {
    let data = &self.data;
    self.gpu.get_or_insert_with(|| {
      let device = &renderer.device;
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: data.as_bytes(),
        usage: wgpu::BufferUsage::INDEX,
      })
    });
  }

  pub fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
    let gpu = self.gpu.as_ref().unwrap();
    pass.set_index_buffer(gpu.slice(..), self.data.index_format());
  }
}
