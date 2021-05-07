use sceno::{ModelHandle, SceneModelCreator};

use crate::{
  MaterialHandle, MeshHandle, NormalizedVec3, RainRayGeometry, RainrayMaterial, RainrayScene,
  Scene, Vec3,
};

pub struct Model {
  pub geometry: MeshHandle,
  pub material: MaterialHandle,
}

impl Model {
  pub fn new<M, G>(scene: &mut Scene, geometry: G, material: M) -> Self
  where
    M: RainrayMaterial,
    G: RainRayGeometry,
  {
    let geometry = scene.meshes.insert(Box::new(geometry));
    let material = scene.materials.insert(Box::new(material));
    Model { geometry, material }
  }
}

impl<M, G> SceneModelCreator<RainrayScene> for (G, M)
where
  M: RainrayMaterial + RainrayMaterial,
  G: RainRayGeometry,
{
  fn create_model(self, scene: &mut sceno::Scene<RainrayScene>) -> ModelHandle<RainrayScene> {
    let model = Model::new(scene, self.0, self.1);
    scene.create_model(model)
  }
}

pub struct BSDFSampleResult {
  pub light_dir: ImportanceSampled<NormalizedVec3>,
  pub bsdf: Vec3,
}

pub struct ImportanceSampled<T> {
  pub sample: T,
  pub pdf: f32,
}
