use rendiation_algebra::*;

use crate::{SceneTextureCube, WebGPUBackground};

pub trait Background: WebGPUBackground {}

impl Background for SolidBackground {}

pub struct SolidBackground {
  pub intensity: Vec3<f32>,
}

impl Default for SolidBackground {
  fn default() -> Self {
    Self {
      intensity: Vec3::new(0.6, 0.6, 0.6),
    }
  }
}

impl SolidBackground {
  pub fn black() -> Self {
    Self {
      intensity: Vec3::splat(0.0),
    }
  }
}

pub struct EnvMapBackground {
  pub texture: SceneTextureCube,
}
