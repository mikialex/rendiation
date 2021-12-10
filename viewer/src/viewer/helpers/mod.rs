use rendiation_algebra::*;
use rendiation_renderable_mesh::mesh::{LineList, NoneIndexedMesh};

pub mod axis;

pub type HelperLineMesh = NoneIndexedMesh<ColoredLineVertex, LineList>;

#[repr(C)]
pub struct ColoredLineVertex {
  pub position: Vec3<f32>,
  pub color: Vec4<f32>,
}

impl rendiation_webgpu::VertexBufferSourceType for ColoredLineVertex {
  fn vertex_layout() -> rendiation_webgpu::VertexBufferLayoutOwned {
    rendiation_webgpu::VertexBufferLayoutOwned {
      array_stride: std::mem::size_of::<Self>() as u64,
      step_mode: wgpu::VertexStepMode::Vertex,
      attributes: vec![
        wgpu::VertexAttribute {
          format: wgpu::VertexFormat::Float32x3,
          offset: 0,
          shader_location: 0,
        },
        wgpu::VertexAttribute {
          format: wgpu::VertexFormat::Float32x4,
          offset: 4 * 3,
          shader_location: 1,
        },
      ],
    }
  }

  fn get_shader_header() -> &'static str {
    r#"
      [[location(0)]] position: vec3<f32>,
      [[location(1)]] color: vec4<f32>,
    "#
  }
}
