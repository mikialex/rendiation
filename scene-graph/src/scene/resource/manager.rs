use crate::{
  Arena, SceneGeometry, SceneGraphBackEnd, SceneShading, SceneShadingParameterGroup, Uniform,
};

pub struct ResourceManager<T: SceneGraphBackEnd> {
  pub geometries: Arena<SceneGeometry<T>>,
  pub shadings: Arena<SceneShading<T>>,
  pub shading_parameter_groups: Arena<SceneShadingParameterGroup<T>>,
  pub uniforms: Arena<Uniform<T>>,
}

impl<T: SceneGraphBackEnd> ResourceManager<T> {
  pub fn new() -> Self {
    Self {
      geometries: Arena::new(),
      shadings: Arena::new(),
      shading_parameter_groups: Arena::new(),
      uniforms: Arena::new()
    }
  }
}
