use crate::*;

#[derive(Copy, Clone)]
pub struct UnitCircle;

impl ParametricCurve2D for UnitCircle {
  fn position(&self, position: f32) -> Vec2<f32> {
    let (s, c) = (position * f32::PI() * 2.).sin_cos();
    Vec2::new(c, s)
  }
}

/// Default is a width height 1. start at origin. XY axis plane.
#[derive(Copy, Clone)]
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

  fn normal_dir(&self, position: Vec2<f32>) -> Vec3<f32> {
    self.position(position)
  }
}

#[derive(Copy, Clone)]
pub struct LineSegment2D {
  pub start: Vec2<f32>,
  pub end: Vec2<f32>,
}

impl ParametricCurve2D for LineSegment2D {
  fn position(&self, position: f32) -> Vec2<f32> {
    self.start.lerp(self.end, position)
  }
}

#[derive(Copy, Clone)]
pub struct LineSegment3D {
  pub start: Vec3<f32>,
  pub end: Vec3<f32>,
}

impl ParametricCurve3D for LineSegment3D {
  fn position(&self, position: f32) -> Vec3<f32> {
    self.start.lerp(self.end, position)
  }

  fn normal_dir(&self, _: f32) -> Vec3<f32> {
    Vec3::new(0., 1., 0.)
  }
}
