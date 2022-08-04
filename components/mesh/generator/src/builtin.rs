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
    let (u_sin, u_cos) = position.x.sin_cos();
    let (v_sin, v_cos) = position.y.sin_cos();
    Vec3::new(u_cos * v_sin, v_cos, u_sin * v_sin)
  }
}

#[derive(Debug, Copy, Clone)]
pub struct TorusParameter {
  radius: f32,
  tube_radius: f32,
}

pub fn torus(param: TorusParameter) -> impl ParametricSurface {
  let TorusParameter {
    radius,
    tube_radius,
  } = param;

  UnitCircle
    .transform_by(Mat3::scale(Vec2::splat(radius)))
    .embed_to_surface(ParametricPlane)
    .make_tube_by(UnitCircle.transform_by(Mat3::scale(Vec2::splat(tube_radius))))
}
