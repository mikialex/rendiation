use crate::*;

pub(crate) struct BrdfEval {
  pub value: f32,
  pub pdf: f32,
}

pub(crate) trait Brdf {
  fn eval(v: Vec3<f32>, l: Vec3<f32>, alpha: f32) -> BrdfEval;
  fn sample(v: Vec3<f32>, l: Vec3<f32>, alpha: f32, u1: f32, u2: f32) -> Vec3<f32>;
}
