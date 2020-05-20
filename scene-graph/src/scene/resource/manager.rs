
use crate::{SceneShading, Arena, SceneGeometry, SceneShadingParameterGroup};

pub trait SceneGraphBackEnd{
  type Renderer;
  type Shading;
  type ShadingParameterGroup;
  type IndexBuffer;
  type VertexBuffer;
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

  // pub fn add_bindgroup(&mut self, shading_params: WGPUBindGroup) -> Index {
  //   self.bindgroups.insert(shading_params)
  // }

  // pub fn get_bindgroup_mut(&mut self, index: Index) -> &mut WGPUBindGroup {
  //   self.bindgroups.get_mut(index).unwrap()
  // }

  // pub fn get_bindgroup(&self, index: Index) -> &WGPUBindGroup {
  //   self.bindgroups.get(index).unwrap()
  // }
}
