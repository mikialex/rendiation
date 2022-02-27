use rendiation_algebra::*;
use rendiation_texture::TextureSampler;

use crate::{ResourceWrapped, SceneTexture2D};

pub type MaterialInner<T> = ResourceWrapped<T>;

#[derive(Clone)]
pub struct PhysicalMaterial {
  pub albedo: Vec3<f32>,
  pub sampler: TextureSampler,
  pub texture: SceneTexture2D,
}

#[derive(Clone)]
pub struct FlatMaterial {
  pub color: Vec4<f32>,
}
