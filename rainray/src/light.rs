use crate::math::*;

pub trait Light {}

#[derive(Debug, Clone, Copy)]
pub struct PointLight {
  pub position: Vec3,
  pub color: Vec3,
}
