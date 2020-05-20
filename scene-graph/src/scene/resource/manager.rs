use crate::{
  Arena, SceneGeometry, SceneShading, SceneShadingDescriptor, SceneShadingParameterGroup,
};

pub trait SceneGraphBackEnd {
  // resource type injection
  type Renderer;
  type Shading;
  type ShadingParameterGroup;
  type IndexBuffer;
  type VertexBuffer;

  // resource type middle layer translation
  fn create_shading(shading_desc: &SceneShadingDescriptor) -> Self::Shading;
}

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
