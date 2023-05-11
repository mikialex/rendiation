use std::ops::{Add, AddAssign};

use rendiation_algebra::*;
use rendiation_geometry::Plane;

// Quadric Error Metrics

#[derive(Debug, Copy, Clone)]
pub struct QEM {
  mat: Mat4<f32>,
}

impl QEM {
  pub fn zero() -> Self {
    QEM { mat: Mat4::zero() }
  }

  pub fn compute_optimal_position(&self) -> Option<Vec3<f32>> {
    let mut mat = self.mat.clone();
    mat.c1 = 0.0;
    mat.c2 = 0.0;
    mat.c3 = 0.0;
    mat.c4 = 1.0;
    mat
      .inverse()
      .map(|m| (m * Vec4::new(0.0, 0.0, 0.0, 1.0)).xyz())
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

impl Add for QEM {
  type Output = Self;

  fn add(self, b: Self) -> Self {
    Self {
      mat: self.mat + b.mat,
    }
  }
}

impl AddAssign for QEM {
  fn add_assign(&mut self, rhs: Self) {
    *self = *self + rhs
  }
}
