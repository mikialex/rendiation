use rendiation_algebra::Vec3;

use crate::*;

pub struct Model {
  pub shape: Box<dyn Shape>,
  pub material: Box<dyn Material>,
}

impl Model {
  pub fn new<M, G>(shape: G, material: M) -> Self
  where
    M: Material,
    G: Shape,
  {
    let shape = Box::new(shape);
    let material = Box::new(material);
    Model { shape, material }
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
