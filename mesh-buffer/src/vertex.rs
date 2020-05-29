use crate::geometry::{HashAbleByConversion};
use rendiation_math::*;
use std::{
  hash::{Hash},
  mem,
};
use rendiation_math_entity::PositionedPoint;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
  pub position: Vec3<f32>,
  pub normal: Vec3<f32>,
  pub uv: Vec2<f32>,
}

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

impl PositionedPoint for Vertex {
  fn position(&self) -> Vec3<f32> {
    self.position
  }
}

impl PartialEq for Vertex {
  fn eq(&self, other: &Self) -> bool {
    self.position == other.position && self.normal == other.normal && self.uv == other.uv
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
