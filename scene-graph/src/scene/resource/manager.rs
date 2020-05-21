use crate::{
  Arena, SceneGeometry, SceneShading, SceneGraphBackEnd, SceneShadingParameterGroup,
};

pub struct ResourceManager<T: SceneGraphBackEnd> {
  pub geometries: Arena<SceneGeometry<T>>,
  pub shadings: Arena<SceneShading<T>>,
  pub shading_parameter_groups: Arena<SceneShadingParameterGroup<T>>,
}

impl<T: SceneGraphBackEnd> ResourceManager<T> {
  pub fn new() -> Self {
    Self {
      geometries: Arena::new(),
      shadings: Arena::new(),
      shading_parameter_groups: Arena::new(),
    }
  }
}
