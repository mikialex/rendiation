use std::hash::Hash;

use rendiation_algebra::*;
use rendiation_geometry::Positioned;

use crate::{AttributeSemantic, AttributeVertex};

#[repr(C)]
#[derive(Clone, Copy, Debug, rendiation_shader_api::ShaderVertex, PartialEq, Default)]
pub struct CommonVertex {
  #[semantic(GeometryPosition)]
  pub position: Vec3<f32>,

  #[semantic(GeometryNormal)]
  pub normal: Vec3<f32>,

  #[semantic(GeometryUV)]
  pub uv: Vec2<f32>,
}

unsafe impl bytemuck::Zeroable for CommonVertex {}
unsafe impl bytemuck::Pod for CommonVertex {}

impl AttributeVertex for CommonVertex {
  fn layout(&self) -> Vec<AttributeSemantic> {
    vec![
      AttributeSemantic::Positions,
      AttributeSemantic::Normals,
      AttributeSemantic::TexCoords(0),
    ]
  }

  fn write(self, target: &mut [Vec<u8>]) {
    target[0].extend(bytemuck::bytes_of(&self.position));
    target[1].extend(bytemuck::bytes_of(&self.normal));
    target[2].extend(bytemuck::bytes_of(&self.uv));
  }
}

impl Hash for CommonVertex {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.position.x.to_bits().hash(state);
    self.position.y.to_bits().hash(state);
    self.position.z.to_bits().hash(state);
    self.normal.x.to_bits().hash(state);
    self.normal.y.to_bits().hash(state);
    self.normal.z.to_bits().hash(state);
    self.uv.x.to_bits().hash(state);
    self.uv.y.to_bits().hash(state);
  }
}
impl Eq for CommonVertex {}

impl Positioned for CommonVertex {
  type Position = Vec3<f32>;

  fn position(&self) -> Self::Position {
    self.position
  }
  fn mut_position(&mut self) -> &mut Self::Position {
    &mut self.position
  }
}

impl CommonVertex {
  pub fn new(position: Vec3<f32>, normal: Vec3<f32>, uv: Vec2<f32>) -> Self {
    CommonVertex {
      position,
      normal,
      uv,
    }
  }
}

pub fn vertex(pos: [f32; 3], _: [f32; 3], tc: [f32; 2]) -> CommonVertex {
  CommonVertex {
    position: Vec3::new(pos[0], pos[1], pos[2]),
    normal: Vec3::new(0.0, 1.0, 0.0),
    uv: Vec2::new(tc[0], tc[1]),
  }
}
