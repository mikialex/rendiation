use rendiation_geometry::{IntersectAble, Ray3};

use crate::{
  material::Material, Intersection, NormalizedVec3, PossibleIntersection, RainRayGeometry, Vec3,
};

pub struct Model<M, G> {
  pub geometry: G,
  pub material: M,
}

impl<M, G> Model<M, G>
where
  M: Material<G>,
  G: RainRayGeometry,
{
  pub fn new(geometry: G, material: M) -> Self {
    Model { geometry, material }
  }
}

impl<M, G: IntersectAble<Ray3, PossibleIntersection>> IntersectAble<Ray3, PossibleIntersection>
  for Model<M, G>
{
  fn intersect(&self, ray: &Ray3, param: &()) -> PossibleIntersection {
    self.geometry.intersect(ray, param)
  }
}

impl<M, G: RainRayGeometry> RainRayGeometry for Model<M, G> {}

impl<M, G> RainrayModel for Model<M, G>
where
  M: 'static + Sync + Send + Material<G>,
  G: RainRayGeometry + 'static + Sync + Send,
{
  fn sample_light_dir(
    &self,
    view_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> NormalizedVec3 {
    self
      .material
      .sample_light_dir(view_dir, intersection, &self.geometry)
  }

  fn pdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> f32 {
    self
      .material
      .pdf(view_dir, light_dir, intersection, &self.geometry)
  }

  fn bsdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> Vec3 {
    self
      .material
      .bsdf(view_dir, light_dir, intersection, &self.geometry)
  }
}

pub trait RainrayModel: Sync + Send + 'static + RainRayGeometry {
  /// sample the light input dir with brdf importance
  fn sample_light_dir(
    &self,
    view_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> NormalizedVec3;
  fn pdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> f32;
  fn bsdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
  ) -> Vec3;
}
