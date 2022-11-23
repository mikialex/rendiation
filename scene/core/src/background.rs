use crate::*;

pub enum SceneBackGround {
  Solid(SolidBackground),
  Env(EnvMapBackground),
  Foreign(Box<dyn ForeignImplemented>),
}

impl Clone for SceneBackGround {
  fn clone(&self) -> Self {
    match self {
      Self::Solid(arg0) => Self::Solid(*arg0),
      Self::Env(arg0) => Self::Env(arg0.clone()),
      Self::Foreign(arg0) => Self::Foreign(dyn_clone::clone_box(
        arg0.as_ref() as &dyn ForeignImplemented
      )),
    }
  }
}

#[derive(Clone, Copy)]
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

#[derive(Clone)]
pub struct EnvMapBackground {
  pub texture: SceneTextureCube,
}
