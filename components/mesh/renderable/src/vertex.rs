use crate::mesh::HashAbleByConversion;
use rendiation_algebra::*;
use rendiation_geometry::Positioned;
use std::{hash::Hash, mem};

#[repr(C)]
#[derive(Clone, Copy, soa_derive::StructOfArray, Debug)]
#[cfg_attr(feature = "shader", derive(shadergraph::ShaderVertex))]
pub struct Vertex {
  #[cfg_attr(feature = "shader", semantic(GeometryLocalSpacePosition))]
  pub position: Vec3<f32>,

  #[cfg_attr(feature = "shader", semantic(GeometryLocalSpaceNormal))]
  pub normal: Vec3<f32>,

  #[cfg_attr(feature = "shader", semantic(GeometryUV))]
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

impl Positioned for Vertex {
  type Position = Vec3<f32>;

  fn position(&self) -> &Self::Position {
    &self.position
  }
  fn mut_position(&mut self) -> &mut Self::Position {
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
