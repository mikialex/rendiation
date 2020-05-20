
use crate::{SceneShading, Arena, SceneGeometry};

pub trait SceneGraphBackEnd{
  type Renderer;
  type Geometry;
  type Shading;
  type ShadingParameterGroup;
}

pub struct ResourceManager<T: SceneGraphBackEnd> {
  pub geometries: Arena<SceneGeometry<T>>,
  // vertex_buffers: Arena<Buffer>,
  pub shadings: Arena<SceneShading<T>>,
  // shading_parameter_group: Arena<WGPUBindGroup>,
}

impl<T: SceneGraphBackEnd> ResourceManager<T> {
  pub fn new() -> Self {
    Self {
      geometries: Arena::new(),
      shadings: Arena::new(),
      // bindgroups: Arena::new(),
    }
  }

  // pub fn add_geometry(&mut self, geometry: impl Geometry) -> Index {
  //   self.geometries.insert(Box::new(geometry))
  // }

  // pub fn get_geometry_mut(&mut self, index: Index) -> &mut dyn Geometry {
  //   self.geometries.get_mut(index).unwrap().as_mut()
  // }

  // pub fn get_geometry(&self, index: Index) -> &dyn Geometry {
  //   self.geometries.get(index).unwrap().as_ref()
  // }

  // pub fn delete_geometry(&mut self, index: Index) {
  //   self.geometries.remove(index);
  // }


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
