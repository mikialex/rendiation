use rendiation_algebra::Vec3;

use crate::*;

pub struct Model {
  pub geometry: Box<dyn RainRayGeometry>,
  pub material: Box<dyn RainrayMaterial>,
}

impl Model {
  pub fn new<M, G>(geometry: G, material: M) -> Self
  where
    M: RainrayMaterial,
    G: RainRayGeometry,
  {
    let geometry = Box::new(geometry);
    let material = Box::new(material);
    Model { geometry, material }
  }
}

// impl<M, G> SceneModelCreator<RainrayScene> for (G, M)
// where
//   M: RainrayMaterial + RainrayMaterial,
//   G: RainRayGeometry,
// {
//   fn create_model(self, scene: &mut sceno::Scene<RainrayScene>) -> ModelHandle<RainrayScene> {
//     let model = Model::new(scene, self.0, self.1);
//     scene.create_model(model)
//   }
// }

pub struct BSDFSampleResult {
  pub light_dir: ImportanceSampled<NormalizedVec3<f32>>,
  pub bsdf: Vec3<f32>,
}

pub struct ImportanceSampled<T> {
  pub sample: T,
  pub pdf: f32,
}
