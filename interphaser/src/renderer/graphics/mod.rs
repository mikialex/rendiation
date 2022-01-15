use rendiation_algebra::*;
use webgpu::{VertexBufferLayoutOwned, VertexBufferSourceType};

#[derive(Debug, Copy, Clone)]
pub struct UIVertex {
  position: Vec2<f32>,
  uv: Vec2<f32>,
  color: Vec4<f32>,
}
unsafe impl bytemuck::Zeroable for UIVertex {}
unsafe impl bytemuck::Pod for UIVertex {}

pub fn vertex(position: (f32, f32), uv: (f32, f32), color: (f32, f32, f32, f32)) -> UIVertex {
  UIVertex {
    position: position.into(),
    uv: uv.into(),
    color: color.into(),
  }
}

impl VertexBufferSourceType for UIVertex {
  fn vertex_layout() -> VertexBufferLayoutOwned {
    webgpu::VertexBufferLayout {
      array_stride: std::mem::size_of::<UIVertex>() as u64,
      step_mode: webgpu::VertexStepMode::Vertex,
      attributes: &webgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x4,
      ],
    }
    .into()
  }

  fn get_shader_header() -> &'static str {
    r#"
      [[location(0)]] position: vec2<f32>,
      [[location(1)]] uv: vec2<f32>,
      [[location(2)]] color: vec4<f32>,
    "#
  }
}
