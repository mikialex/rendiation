use crate::{WGPUBindGroup, WGPUBuffer, WGPUPipeline, WGPURenderer};
use generational_arena::{Arena, Index};
use std::any::Any;

pub trait Geometry: Any {
  fn update_gpu(&mut self, renderer: &mut WGPURenderer);
  fn get_gpu_index_buffer(&self) -> &WGPUBuffer;
  fn get_gpu_geometry_buffer(&self) -> &WGPUBuffer;
}

pub trait Shading: Any {
  fn get_gpu_pipeline(&self) -> &WGPUPipeline;
}

pub trait ShadingParamGroup: Any {
  fn get_gpu_bindgroup(&self) -> &WGPUBindGroup;
}

pub struct ResourceManager {
  geometries: Arena<Box<dyn Geometry>>,
  shadings: Arena<Box<dyn Shading>>,
  shading_params: Arena<Box<dyn ShadingParamGroup>>,
}

impl ResourceManager {
  pub fn new() -> Self {
    Self {
      geometries: Arena::new(),
      shadings: Arena::new(),
      shading_params: Arena::new(),
    }
  }

  pub fn add_geometry(&mut self, geometry: impl Geometry) -> Index {
    self.geometries.insert(Box::new(geometry))
  }

  pub fn get_geometry(&mut self, index: Index) -> &mut dyn Geometry {
    self.geometries.get_mut(index).unwrap().as_mut()
  }

  pub fn delete_geometry(&mut self, index: Index) {
    self.geometries.remove(index);
  }

  pub fn add_shading(&mut self, shading: impl Shading) -> Index {
    self.shadings.insert(Box::new(shading))
  }

  pub fn get_shading(&mut self, index: Index) -> &mut dyn Shading {
    self.shadings.get_mut(index).unwrap().as_mut()
  }

  pub fn add_shading_params(&mut self, shading_params: impl ShadingParamGroup) -> Index {
    self.shading_params.insert(Box::new(shading_params))
  }

  pub fn get_shading_params(&mut self, index: Index) -> &mut dyn ShadingParamGroup {
    self.shading_params.get_mut(index).unwrap().as_mut()
  }
}
