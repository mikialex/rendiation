use std::any::Any;

use crate::{math::*, RayTraceScene, Scene};

pub mod mesh;
pub use mesh::*;
use rendiation_algebra::{IntoNormalizedVector, Mat4, SpaceEntity, Vec2, Vec3};

pub trait RainRayGeometry: Sync + Send + 'static {
  fn as_any(&self) -> &dyn Any;

  fn intersect<'a>(&self, ray: Ray3, scene: &RayTraceScene<'a>) -> PossibleIntersection;

  fn has_any_intersect<'a>(&self, ray: Ray3, scene: &RayTraceScene<'a>) -> bool {
    self.intersect(ray, scene).0.is_some()
  }

  fn get_bbox<'a>(&self, _scene: &'a Scene) -> Option<Box3> {
    None
  }
}

pub struct Intersection {
  pub position: Vec3<f32>,
  pub geometric_normal: NormalizedVec3<f32>,
  pub shading_normal: NormalizedVec3<f32>,
  pub uv: Option<Vec2<f32>>,
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

pub struct PossibleIntersection(pub Option<Intersection>);

impl RainRayGeometry for Sphere {
  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn intersect<'a>(&self, ray: Ray3, _: &RayTraceScene<'a>) -> PossibleIntersection {
    let result: Nearest<HitPoint3D> = ray.intersect(self, &());
    PossibleIntersection(result.0.map(|near| {
      let normal = (near.position - self.center).into_normalized();
      Intersection {
        position: near.position,
        geometric_normal: normal,
        shading_normal: normal,
        uv: None,
      }
    }))
  }

  fn get_bbox<'a>(&self, _scene: &'a Scene) -> Option<Box3> {
    self.to_bounding().into()
  }
}

impl RainRayGeometry for Plane {
  fn as_any(&self) -> &dyn std::any::Any {
    self
  }

  fn intersect<'a>(&self, ray: Ray3, _: &RayTraceScene<'a>) -> PossibleIntersection {
    let result: Nearest<HitPoint3D> = ray.intersect(self, &());
    PossibleIntersection(result.0.map(|near| Intersection {
      position: near.position,
      geometric_normal: self.normal,
      shading_normal: self.normal,
      uv: None,
    }))
  }
}
