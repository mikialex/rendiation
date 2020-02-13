use crate::renderer::pipeline::VertexProvider;
use rendiation_math::*;

#[derive(Clone, Copy)]
pub struct Vertex {
  pub position: Vec3<f32>,
  pub normal: Vec3<f32>,
  pub uv: Vec2<f32>,
}

impl<'a> VertexProvider<'a> for Vertex {
  fn get_buffer_layout_descriptor() -> wgpu::VertexBufferDescriptor<'a> {
    use std::mem;
    wgpu::VertexBufferDescriptor {
      stride: mem::size_of::<Self>() as wgpu::BufferAddress,
      step_mode: wgpu::InputStepMode::Vertex,
      attributes: &[
        wgpu::VertexAttributeDescriptor {
          format: wgpu::VertexFormat::Float3,
          offset: 0,
          shader_location: 0,
        },
        wgpu::VertexAttributeDescriptor {
          format: wgpu::VertexFormat::Float3,
          offset: 4 * 3,
          shader_location: 1,
        },
        wgpu::VertexAttributeDescriptor {
          format: wgpu::VertexFormat::Float2,
          offset: 4 * 3 + 4 * 3,
          shader_location: 2,
        },
      ],
    }
  }
}

pub fn vertex(pos: [f32; 3],n: [f32; 3], tc: [f32; 2]) -> Vertex {
  Vertex {
    position: Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32),
    normal: Vec3::new(0.0, 1.0, 0.0),
    uv: Vec2::new(tc[0] as f32, tc[1] as f32),
  }
}
