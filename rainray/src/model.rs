use std::{any::Any, marker::PhantomData};

use rendiation_geometry::{Box3, IntersectAble, Ray3};

use crate::{
  material::Material, Intersection, MaterialHandle, MeshHandle, NormalizedVec3,
  PossibleIntersection, RainRayGeometry, Scene, Vec3,
};

pub struct Model<M, G> {
  mesh_phantom: PhantomData<G>,
  mat_phantom: PhantomData<M>,
  pub geometry: MeshHandle,
  pub material: MaterialHandle,
}

impl<M, G> Model<M, G>
where
  M: Material<G> + RainrayMaterial,
  G: RainRayGeometry,
{
  pub fn new(scene: &mut Scene, geometry: G, material: M) -> Self {
    let geometry = scene.meshes.insert(Box::new(geometry));
    let material = scene.materials.insert(Box::new(material));
    Model {
      geometry,
      material,
      mesh_phantom: PhantomData,
      mat_phantom: PhantomData,
    }
  }

  pub fn downcast<'a>(&self, scene: &'a Scene) -> (&'a M, &'a G) {
    let material = scene
      .materials
      .get(self.material)
      .unwrap()
      .as_any()
      .downcast_ref::<M>()
      .unwrap();
    let geometry = scene
      .meshes
      .get(self.geometry)
      .unwrap()
      .as_any()
      .downcast_ref::<G>()
      .unwrap();
    (material, geometry)
  }
}

impl<M, G: IntersectAble<Ray3, PossibleIntersection>> IntersectAble<Ray3, PossibleIntersection>
  for Model<M, G>
{
  fn intersect(&self, ray: &Ray3, param: &()) -> PossibleIntersection {
    self.geometry.intersect(ray, param)
  }
}

impl<M: RainrayMaterial, G: RainRayGeometry> RainRayGeometry for Model<M, G> {
  fn get_bbox(&self) -> Option<Box3> {
    self.geometry.get_bbox()
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}

impl<M, G> RainrayModel for Model<M, G>
where
  M: 'static + Sync + Send + Material<G> + RainrayMaterial,
  G: RainRayGeometry + 'static + Sync + Send,
{
  fn sample_light_dir(
    &self,
    view_dir: NormalizedVec3,
    intersection: &Intersection,
    scene: &Scene,
  ) -> NormalizedVec3 {
    let (material, geometry) = self.downcast(scene);
    material.sample_light_dir(view_dir, intersection, geometry)
  }

  fn pdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
    scene: &Scene,
  ) -> f32 {
    let (material, geometry) = self.downcast(scene);
    material.pdf(view_dir, light_dir, intersection, geometry)
  }

  fn bsdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
    scene: &Scene,
  ) -> Vec3 {
    let (material, geometry) = self.downcast(scene);
    material.bsdf(view_dir, light_dir, intersection, geometry)
  }
}

pub trait RainrayModel: Sync + Send + 'static + RainRayGeometry {
  /// sample the light input dir with brdf importance
  fn sample_light_dir(
    &self,
    view_dir: NormalizedVec3,
    intersection: &Intersection,
    scene: &Scene,
  ) -> NormalizedVec3;
  fn pdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
    scene: &Scene,
  ) -> f32;
  fn bsdf(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
    scene: &Scene,
  ) -> Vec3;
}

pub trait RainrayMaterial: Any + Sync + Send {
  fn as_any(&self) -> &dyn Any;
}
