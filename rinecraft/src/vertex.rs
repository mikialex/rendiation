use crate::renderer::pipeline::VertexProvider;
use rendiation_math::*;
use rendiation::*;

#[derive(Clone, Copy)]
pub struct Vertex {
  position: Vec3<f32>,
  uv: Vec2<f32>,
}

impl<'a> VertexProvider<'a> for Vertex {
  fn get_buffer_layout_discriptor() -> wgpu::VertexBufferDescriptor<'a> {
    use std::mem;
    wgpu::VertexBufferDescriptor {
      stride: mem::size_of::<Self>() as wgpu::BufferAddress,
      step_mode: wgpu::InputStepMode::Vertex,
      attributes: &[
        wgpu::VertexAttributeDescriptor {
          format: wgpu::VertexFormat::Float4,
          offset: 0,
          shader_location: 0,
        },
        wgpu::VertexAttributeDescriptor {
          format: wgpu::VertexFormat::Float2,
          offset: 4 * 3,
          shader_location: 1,
        },
      ],
    }
  }
}

pub fn vertex(pos: [i8; 3], tc: [i8; 2]) -> Vertex {
  Vertex {
    position: Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32),
    uv: Vec2::new(tc[0] as f32, tc[1] as f32),
  }
}
