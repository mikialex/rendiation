use rendiation_algebra::Vec3;

pub trait Light {}

#[derive(Debug, Clone, Copy)]
pub struct PointLight {
  pub intensity: Vec3<f32>,
}

#[derive(Debug, Clone, Copy)]
pub struct SpotLight {}
