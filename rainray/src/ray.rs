use crate::math::*;
use rendiation_math::{InnerProductSpace, IntoNormalizedVector, Vector};

pub trait RainRayGeometry: Send + Sync + IntersectAble<Ray3, PossibleIntersection> {}

pub struct Intersection {
  pub distance: f32,
  pub hit_position: Vec3,
  pub hit_normal: NormalizedVec3,
}

pub struct PossibleIntersection(pub Option<Intersection>);

impl IntersectAble<Ray3, PossibleIntersection> for Sphere {
  fn intersect(&self, ray: &Ray3, param: &()) -> PossibleIntersection {
    let result: NearestPoint3D = ray.intersect(self, param);
    PossibleIntersection(result.0.map(|near| Intersection {
      distance: near.distance,
      hit_position: near.position,
      hit_normal: (near.position - self.center).into_normalized(),
    }))
  }
}
impl RainRayGeometry for Sphere {}

impl IntersectAble<Ray3, PossibleIntersection> for Plane {
  fn intersect(&self, ray: &Ray3, param: &()) -> PossibleIntersection {
    let result: NearestPoint3D = ray.intersect(self, param);
    PossibleIntersection(result.0.map(|near| Intersection {
      distance: near.distance,
      hit_position: near.position,
      hit_normal: self.normal,
    }))
  }
}
impl RainRayGeometry for Plane {}
