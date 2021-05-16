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

pub struct BSDFSampleResult {
  pub light_dir: ImportanceSampled<NormalizedVec3<f32>>,
  pub bsdf: Vec3<f32>,
}

pub struct ImportanceSampled<T> {
  pub sample: T,
  pub pdf: f32,
}
