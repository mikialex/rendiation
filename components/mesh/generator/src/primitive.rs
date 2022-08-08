use crate::*;

pub struct UnitCircle;

impl ParametricCurve2D for UnitCircle {
  fn position(&self, position: f32) -> Vec2<f32> {
    let (s, c) = position.sin_cos();
    Vec2::new(c, s)
  }
}

/// Default is a width height 1. start at origin. XY axis plane.
pub struct ParametricPlane;

impl ParametricSurface for ParametricPlane {
  fn position(&self, position: Vec2<f32>) -> Vec3<f32> {
    Vec3::new(position.x, position.y, 0.)
  }
}

pub struct UVSphere;

impl ParametricSurface for UVSphere {
  fn position(&self, position: Vec2<f32>) -> Vec3<f32> {
    let (u_sin, u_cos) = (position.x * f32::PI() * 2.).sin_cos();
    let (v_sin, v_cos) = (position.y * f32::PI()).sin_cos();
    Vec3::new(u_cos * v_sin, v_cos, u_sin * v_sin)
  }
}

pub struct UintLine3D;

impl ParametricCurve3D for UnitCircle {
  fn position(&self, position: f32) -> Vec3<f32> {
    Vec3::new(0., position, 0.)
  }

  fn normal(&self, _: f32) -> Vec3<f32> {
    Vec3::new(1., 0., 0.)
  }
}
