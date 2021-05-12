use crate::geometry::HashAbleByConversion;
use rendiation_algebra::*;
use std::{
  hash::Hash,
  mem,
  ops::{Deref, DerefMut},
};

#[repr(C)]
#[cfg_attr(feature = "shadergraph", derive(Geometry))]
#[derive(Clone, Copy, soa_derive::StructOfArray, Debug)]
pub struct Vertex {
  pub position: Vec3<f32>,
  pub normal: Vec3<f32>,
  pub uv: Vec2<f32>,
}

unsafe impl bytemuck::Zeroable for Vertex {}
unsafe impl bytemuck::Pod for Vertex {}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct HashableVertex {
  pub position: Vec3<u32>,
  pub normal: Vec3<u32>,
  pub uv: Vec2<u32>,
}

impl HashAbleByConversion for Vertex {
  type HashAble = HashableVertex;
  fn to_hashable(&self) -> Self::HashAble {
    unsafe { mem::transmute(*self) }
  }
}

impl Deref for Vertex {
  type Target = Vec3<f32>;

  fn deref(&self) -> &Self::Target {
    &self.position
  }
}
impl DerefMut for Vertex {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.position
  }
}

impl Vertex {
  pub fn new(position: Vec3<f32>, normal: Vec3<f32>, uv: Vec2<f32>) -> Self {
    Vertex {
      position,
      normal,
      uv,
    }
  }
}

pub fn vertex(pos: [f32; 3], _: [f32; 3], tc: [f32; 2]) -> Vertex {
  Vertex {
    position: Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32),
    normal: Vec3::new(0.0, 1.0, 0.0),
    uv: Vec2::new(tc[0] as f32, tc[1] as f32),
  }
}

// impl VertexBufferLayoutProvider for Vertex {
//   const DESCRIPTOR: VertexBufferLayout<'static> = VertexBufferLayout {
//     step_mode: InputStepMode::Vertex,
//     array_stride: mem::size_of::<Self>() as u64,
//     attributes: &[
//       VertexAttribute {
//         offset: 0,
//         shader_location: 0, // todo shader location should append by providers before
//         format: VertexFormat::Float3,
//       },
//       VertexAttribute {
//         offset: 4 * 3,
//         shader_location: 1,
//         format: VertexFormat::Float3,
//       },
//       VertexAttribute {
//         offset: 4 * 3 + 4 * 3,
//         shader_location: 2,
//         format: VertexFormat::Float2,
//       },
//     ],
//   };
// }
