use std::{any::Any, marker::PhantomData};

use rendiation_geometry::{Box3, Ray3};
use sceno::{ModelHandle, SceneModelCreator};

use crate::{
  material::Material, Intersection, MaterialHandle, MeshHandle, NormalizedVec3,
  PossibleIntersection, RainRayGeometry, RainrayMaterial, RainrayScene, RayTraceScene, Scene, Vec3,
};

pub struct Model<M, G> {
  mesh_phantom: PhantomData<G>,
  mat_phantom: PhantomData<M>,
  pub geometry: MeshHandle,
  pub material: MaterialHandle,
}

impl<M, G> SceneModelCreator<RainrayScene> for (G, M)
where
  M: Material<G> + RainrayMaterial,
  G: RainRayGeometry,
{
  fn create_model(self, scene: &mut sceno::Scene<RainrayScene>) -> ModelHandle<RainrayScene> {
    let model = Model::boxed(scene, self.0, self.1) as Box<dyn RainrayModel>;
    scene.create_model(model)
  }
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

  pub fn boxed(scene: &mut Scene, geometry: G, material: M) -> Box<Self> {
    Box::new(Self::new(scene, geometry, material))
  }

  pub fn downcast<'a, 'b>(&self, scene: &'a RayTraceScene<'b>) -> (&'a M, &'a G) {
    let material = scene
      .scene
      .materials
      .get(self.material)
      .unwrap()
      .as_any()
      .downcast_ref::<M>()
      .unwrap();
    let geometry = scene
      .scene
      .meshes
      .get(self.geometry)
      .unwrap()
      .as_any()
      .downcast_ref::<G>()
      .unwrap();
    (material, geometry)
  }
}

impl<M: RainrayMaterial + Material<G>, G: RainRayGeometry> RainRayGeometry for Model<M, G> {
  fn get_bbox<'a>(&self, scene: &RayTraceScene) -> Option<Box3> {
    let (_, geometry) = self.downcast(scene);
    geometry.get_bbox(scene)
  }

  fn as_any(&self) -> &dyn Any {
    self
  }

  fn intersect<'a>(&self, ray: Ray3, scene: &RayTraceScene<'a>) -> PossibleIntersection {
    let geometry = scene.scene.meshes.get(self.geometry).unwrap();
    geometry.intersect(ray, scene)
  }
}

impl<M, G> RainrayModel for Model<M, G>
where
  M: 'static + Sync + Send + Material<G> + RainrayMaterial,
  G: RainRayGeometry + 'static + Sync + Send,
{
  fn sample_light_dir<'a>(
    &self,
    view_dir: NormalizedVec3,
    intersection: &Intersection,
    scene: &RayTraceScene<'a>,
  ) -> NormalizedVec3 {
    let (material, geometry) = self.downcast(scene);
    material.sample_light_dir(view_dir, intersection, geometry)
  }

  fn pdf<'a>(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
    scene: &RayTraceScene<'a>,
  ) -> f32 {
    let (material, geometry) = self.downcast(scene);
    material.pdf(view_dir, light_dir, intersection, geometry)
  }

  fn bsdf<'a>(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
    scene: &RayTraceScene<'a>,
  ) -> Vec3 {
    let (material, geometry) = self.downcast(scene);
    material.bsdf(view_dir, light_dir, intersection, geometry)
  }
}

pub trait RainrayModel: Sync + Send + 'static + RainRayGeometry {
  /// sample the light input dir with brdf importance
  fn sample_light_dir<'a>(
    &self,
    view_dir: NormalizedVec3,
    intersection: &Intersection,
    scene: &RayTraceScene<'a>,
  ) -> NormalizedVec3;
  fn pdf<'a>(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
    scene: &RayTraceScene<'a>,
  ) -> f32;
  fn bsdf<'a>(
    &self,
    view_dir: NormalizedVec3,
    light_dir: NormalizedVec3,
    intersection: &Intersection,
    scene: &RayTraceScene<'a>,
  ) -> Vec3;
}
