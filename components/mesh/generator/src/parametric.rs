use crate::*;

const EPS: f32 = 0.0001;

pub trait ParametricSurface {
  fn position(&self, position: Vec2<f32>) -> Vec3<f32>;
  fn normal(&self, position: Vec2<f32>) -> Vec3<f32> {
    let p = self.position(position);
    let u = self.position(position + Vec2::new(EPS, 0.));
    let v = self.position(position + Vec2::new(0., EPS));

    let u = (u - p).normalize();
    let v = (v - p).normalize();
    v.cross(u)
  }
}

pub trait ParametricCurve3D {
  fn position(&self, position: f32) -> Vec3<f32>;
  fn tangent(&self, position: f32) -> Vec3<f32> {
    let p1 = self.position(position);
    let p2 = self.position(position + EPS);
    (p2 - p1).normalize()
  }
  fn normal(&self, position: f32) -> Vec3<f32>;
}

pub trait ParametricCurve2D {
  fn position(&self, position: f32) -> Vec2<f32>;
  fn tangent(&self, position: f32) -> Vec2<f32> {
    let p1 = self.position(position);
    let p2 = self.position(position + EPS);
    (p2 - p1).normalize()
  }
  fn normal(&self, position: f32) -> Vec2<f32> {
    self.tangent(position).perpendicular_cw()
  }
}
