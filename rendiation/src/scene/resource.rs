use crate::{WGPUBindGroup, WGPUBuffer, WGPUPipeline};
use generational_arena::{Arena, Index};
use std::any::Any;

pub trait Geometry: Any {
  fn get_gpu_index_buffer(&self) -> WGPUBuffer;
}

pub trait Shading: Any {
  fn get_gpu_pipeline(&self) -> WGPUPipeline;
}

pub trait ShadingParamGroup: Any {
  fn get_gpu_bindgroup(&self) -> WGPUBindGroup;
}

pub struct ResourceManager {
  geometries: Arena<Box<dyn Geometry>>,
  shadings: Arena<Box<dyn Shading>>,
  shading_params: Arena<Box<dyn ShadingParamGroup>>,
}

impl ResourceManager {
  pub fn add_geometry(&mut self, geometry: impl Geometry) -> Index {
    self.geometries.insert(Box::new(geometry))
  }

  pub fn get_geometry(&mut self, index: Index) -> &mut dyn Geometry {
    self.geometries.get_mut(index).unwrap().as_mut()
  }

  pub fn add_shading(&mut self, shading: impl Shading) -> Index {
    self.shadings.insert(Box::new(shading))
  }

  pub fn get_shading(&mut self, index: Index) -> &mut dyn Shading {
    self.shadings.get_mut(index).unwrap().as_mut()
  }
}
