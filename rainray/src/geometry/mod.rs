use crate::math::*;
use rendiation_algebra::IntoNormalizedVector;

pub mod mesh;
pub use mesh::*;

pub trait RainRayGeometry: IntersectAble<Ray3, PossibleIntersection> {}

pub struct Intersection {
  pub distance: f32,
  pub position: Vec3,
  pub geometric_normal: NormalizedVec3,
  pub shading_normal: NormalizedVec3,
}

const ORIGIN: f32 = 1.0 / 32.0;
const FLOAT_SCALE: f32 = 1.0 / 65536.0;
const INT_SCALE: f32 = 256.0;

#[inline(always)]
fn float_as_int(f: f32) -> i32 {
  unsafe { std::mem::transmute(f) }
}
#[inline(always)]
fn int_as_float(f: i32) -> f32 {
  unsafe { std::mem::transmute(f) }
}

// Normal points outward for rays exiting the surface, else is flipped.
#[rustfmt::skip]
#[inline(always)]
fn offset_ray(p: Vec3, n: Vec3) -> Vec3 {
  let of_i = n.map(|n| (n * INT_SCALE) as i32);
  let p_i = p.zip(of_i, |p, of_i_p| {
    int_as_float(float_as_int(p) + (if p < 0. { -of_i_p } else { of_i_p }))
  });

   Vec3::new(
     if p.x.abs() < ORIGIN { p.x + FLOAT_SCALE * n.x } else { p_i.x },
     if p.y.abs() < ORIGIN { p.y + FLOAT_SCALE * n.y } else { p_i.y },
     if p.z.abs() < ORIGIN { p.z + FLOAT_SCALE * n.z } else { p_i.z },
   )
}

impl Intersection {
  /// use RTX gem's method to solve self intersection issue caused by float precision;
  pub fn adjust_hit_position(&mut self) {
    // self.hit_position = self.hit_position + self.hit_normal * 0.001;
    self.position = offset_ray(self.position, self.geometric_normal.value)
  }
}

pub struct PossibleIntersection(pub Option<Intersection>);

impl IntersectAble<Ray3, PossibleIntersection> for Sphere {
  fn intersect(&self, ray: &Ray3, param: &()) -> PossibleIntersection {
    let result: Nearest<HitPoint3D> = ray.intersect(self, param);
    PossibleIntersection(result.0.map(|near| {
      let normal = (near.position - self.center).into_normalized();
      Intersection {
        distance: near.distance,
        position: near.position,
        geometric_normal: normal,
        shading_normal: normal,
      }
    }))
  }
}
impl RainRayGeometry for Sphere {}

impl IntersectAble<Ray3, PossibleIntersection> for Plane {
  fn intersect(&self, ray: &Ray3, param: &()) -> PossibleIntersection {
    let result: Nearest<HitPoint3D> = ray.intersect(self, param);
    PossibleIntersection(result.0.map(|near| Intersection {
      distance: near.distance,
      position: near.position,
      geometric_normal: self.normal,
      shading_normal: self.normal,
    }))
  }
}
impl RainRayGeometry for Plane {}
