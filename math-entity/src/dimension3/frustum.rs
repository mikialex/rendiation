use crate::plane::Plane;
use crate::DistanceTo;
use crate::IntersectAble;
use crate::{intersect_reverse, sphere::Sphere, Box3};
use rendiation_math::*;

#[derive(Clone)]
pub struct Frustum<T: Scalar = f32> {
  planes: [Plane<T>; 6],
}

impl Default for Frustum {
  fn default() -> Self {
    Self::new()
  }
}

impl<T: Scalar> Frustum<T> {
  pub fn new() -> Self {
    Self {
      planes: [Plane::new(Vec3::splat(T::one()).into_normalized(), T::one()); 6],
    }
  }

  #[rustfmt::skip]
  pub fn set_from_matrix(&mut self, m: Mat4<T>) -> &Self {
    self.planes[0].set_components(m.a4 - m.a1, m.b4 - m.b1, m.c4 - m.c1, m.d4 - m.d1);
    self.planes[1].set_components(m.a4 + m.a1, m.b4 + m.b1, m.c4 + m.c1, m.d4 + m.d1);
    self.planes[2].set_components(m.a4 + m.a2, m.b4 + m.b2, m.c4 + m.c2, m.d4 + m.d2);
    self.planes[3].set_components(m.a4 - m.a2, m.b4 - m.b2, m.c4 - m.c2, m.d4 - m.d2);
    self.planes[4].set_components(m.a4 - m.a3, m.b4 - m.b3, m.c4 - m.c3, m.d4 - m.d3);
    self.planes[5].set_components(m.a4 + m.a3, m.b4 + m.b3, m.c4 + m.c3, m.d4 + m.d3);
    self
  }
}

intersect_reverse!(Sphere, bool, (), Frustum);
impl IntersectAble<Sphere, bool> for Frustum {
  fn intersect(&self, sphere: &Sphere, _: &()) -> bool {
    let neg_radius = -sphere.radius;

    for p in &self.planes {
      let distance = p.distance_to(&sphere.center);
      if distance < neg_radius {
        return false;
      }
    }

    true
  }
}

intersect_reverse!(Box3, bool, (), Frustum);
impl IntersectAble<Box3, bool> for Frustum {
  fn intersect(&self, box3: &Box3, _: &()) -> bool {
    for p in &self.planes {
      if p.distance_to(&box3.max_corner(*p.normal)) < 0. {
        return false;
      }
    }

    true
  }
}
