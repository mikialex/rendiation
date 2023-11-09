use std::{any::Any, ops::AddAssign};

mod mesh;
pub use mesh::*;

use crate::*;

pub trait Shape: Sync + Send + 'static + dyn_clone::DynClone {
  fn as_any(&self) -> &dyn Any;

  fn intersect(&self, ray: Ray3) -> Option<Intersection>;

  fn has_any_intersect(&self, ray: Ray3) -> bool {
    self.intersect(ray).is_some()
  }

  fn get_bbox(&self) -> Option<Box3> {
    None
  }

  fn intersect_statistic(&self, _ray: Ray3) -> IntersectionStatistic {
    Default::default()
  }
}

dyn_clone::clone_trait_object!(Shape);

#[derive(Default)]
pub struct IntersectionStatistic {
  pub box3: usize,
  pub sphere: usize,
  pub triangle: usize,
}

impl AddAssign for IntersectionStatistic {
  fn add_assign(&mut self, rhs: Self) {
    self.box3 += rhs.box3;
    self.sphere += rhs.sphere;
    self.triangle += rhs.triangle;
  }
}

pub struct Intersection {
  pub position: Vec3<f32>,
  pub geometric_normal: NormalizedVec3<f32>,
  pub shading_normal: NormalizedVec3<f32>,
  pub uv: Option<Vec2<f32>>,
}

impl IntersectionCtxBase for Intersection {
  fn shading_normal(&self) -> NormalizedVec3<f32> {
    self.shading_normal
  }
}

const ORIGIN: f32 = 1.0 / 32.0;
const FLOAT_SCALE: f32 = 1.0 / 65536.0;
const INT_SCALE: f32 = 256.0;

#[inline(always)]
fn float_as_int(f: f32) -> i32 {
  #[allow(clippy::transmute_float_to_int)]
  unsafe {
    std::mem::transmute(f)
  }
}
#[inline(always)]
fn int_as_float(f: i32) -> f32 {
  #[allow(clippy::transmute_int_to_float)]
  unsafe {
    std::mem::transmute(f)
  }
}

// Normal points outward for rays exiting the surface, else is flipped.
#[rustfmt::skip]
#[inline(always)]
fn offset_ray(p: Vec3<f32>, n: Vec3<f32>) -> Vec3<f32> {
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
    self.position = offset_ray(self.position, self.geometric_normal.value)
  }

  pub fn apply_matrix(&mut self, matrix: Mat4<f32>, normal_matrix: Mat4<f32>) {
    self.position.apply_matrix(matrix);
    self.geometric_normal = self.geometric_normal.transform_direction(normal_matrix);
    self.shading_normal = self.shading_normal.transform_direction(normal_matrix);
  }
}

impl Shape for Sphere {
  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn intersect(&self, ray: Ray3) -> Option<Intersection> {
    let result: OptionalNearest<HitPoint3D> = ray.intersect(self, &());
    result.0.map(|near| {
      let normal = (near.position - self.center).into_normalized();
      Intersection {
        position: near.position,
        geometric_normal: normal,
        shading_normal: normal,
        uv: None,
      }
    })
  }

  fn get_bbox(&self) -> Option<Box3> {
    self.to_bounding().into()
  }
}

impl Shape for Plane {
  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn intersect(&self, ray: Ray3) -> Option<Intersection> {
    let result: OptionalNearest<HitPoint3D> = ray.intersect(self, &());
    result.0.map(|near| Intersection {
      position: near.position,
      geometric_normal: self.normal,
      shading_normal: self.normal,
      uv: None,
    })
  }
}
