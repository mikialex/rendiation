use rendiation_math::{Mat4, Zero};
use rendiation_math_entity::Plane;

// Quadric Error Metrics
pub struct QEM {
  mat: Mat4<f32>,
}

impl QEM {
  pub fn zero() -> Self {
    QEM { mat: Mat4::zero() }
  }
}

#[rustfmt::skip]
impl From<Plane> for QEM {
  fn from(plane: Plane) -> QEM {
    let a = plane.normal.x;
    let b = plane.normal.y;
    let c = plane.normal.z;
    let d = -plane.constant;

    let mat = Mat4::new(
        a*a, a*b, a*c, a*d,
        a*b, b*b, b*c, b*d,
        a*c, b*c, c*c, c*d,
        a*d, b*d, c*d, d*d,
    );
    
    QEM { mat }
  }
}
