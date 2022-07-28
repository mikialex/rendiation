use rendiation_algebra::*;
use rendiation_texture::TextureSampler;

use crate::*;

#[derive(Clone)]
pub struct PhysicalMaterial<S: SceneContent> {
  pub albedo: Vec3<f32>,
  pub sampler: TextureSampler,
  pub texture: SceneTexture2D<S>,
}

#[derive(Clone)]
pub struct FlatMaterial {
  pub color: Vec4<f32>,
}
