use crate::*;

const EPS: f32 = 0.0001;

pub trait ParametricSurface {
  fn position(&self, position: Vec2<f32>) -> Vec3<f32>;
  /// for performance consideration:
  /// - the output is not guaranteed to be normalized
  /// - the implementation should override this method if possible
  fn normal_dir(&self, mut position: Vec2<f32>) -> Vec3<f32> {
    if position.x + EPS >= 1. {
      position.x = 1. - EPS;
    }

    if position.y + EPS >= 1. {
      position.y = 1. - EPS;
    }

    let p = self.position(position);

    let u = self.position(position + Vec2::new(EPS, 0.));
    let v = self.position(position + Vec2::new(0., EPS));

    let u = u - p;
    let v = v - p;
    u.cross(v)
  }
}

pub trait ParametricCurve3D {
  fn position(&self, position: f32) -> Vec3<f32>;
  /// for performance consideration:
  /// - the output is not guaranteed to be normalized
  /// - the implementation should override this method if possible
  fn tangent_dir(&self, position: f32) -> Vec3<f32> {
    let p1 = self.position(position);
    let p2 = self.position(position + EPS);
    p2 - p1
  }
  /// for performance consideration:
  /// - the output is not guaranteed to be normalized
  /// - the implementation should override this method if possible
  fn normal_dir(&self, position: f32) -> Vec3<f32>;
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
