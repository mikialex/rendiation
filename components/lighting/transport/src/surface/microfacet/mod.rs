use crate::*;

pub trait MicroFacetNormalDistribution {
  /// Normal distribution term, the integral needs normalized to 1.
  fn d(&self, n: NormalizedVec3<f32>, h: NormalizedVec3<f32>) -> f32;
}

pub trait MicroFacetGeometricShadow {
  fn g(&self, l: NormalizedVec3<f32>, v: NormalizedVec3<f32>, n: NormalizedVec3<f32>) -> f32;
}

pub trait MicroFacetFresnel {
  fn f(&self, v: NormalizedVec3<f32>, h: NormalizedVec3<f32>) -> Vec3<f32>;
}

pub trait MicroFacetFresnelShader {
  fn f(&self, v_dot_h: Node<f32>) -> Node<Vec3<f32>>;
}

pub struct SimpleShaderFresnel {
  pub f0: Node<Vec3<f32>>,
}

impl MicroFacetFresnelShader for SimpleShaderFresnel {
  fn f(&self, v_dot_h: Node<f32>) -> Node<Vec3<f32>> {
    todo!()
  }
}
