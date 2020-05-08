use crate::{WGPUBindGroup, WGPUBuffer, WGPUPipeline, WGPURenderer};
use generational_arena::{Arena, Index};
use std::{ops::Range, any::Any};

pub trait Geometry: Any {
  fn update_gpu(&mut self, renderer: &mut WGPURenderer);
  fn get_gpu_index_buffer(&self) -> &WGPUBuffer;
  fn get_gpu_vertex_buffer(&self) -> &[WGPUBuffer];
  fn get_draw_range(&self) -> Range<u32>;
}

// maybe not need trait now

// pub trait Shading: Any {
//   fn get_gpu_pipeline(&self) -> &WGPUPipeline;
//   fn get_bindgroup_count(&self) -> usize;
//   fn get_bindgroup(&self, index: usize) -> Index;
// }

pub struct Shading {
  pipeline: WGPUPipeline,
  bindgroup_indices: Vec<Index>,
  // geometry_type
}

impl Shading {
  pub fn new(pipeline: WGPUPipeline) -> Self {
    Self {
      pipeline,
      bindgroup_indices: Vec::new(),
    }
  }

  pub fn set_bindgroup(&mut self, index: Index) {
    self.bindgroup_indices.push(index);
  }

  pub fn get_gpu_pipeline(&self) -> &WGPUPipeline {
    &self.pipeline
  }
  pub fn get_bindgroup_count(&self) -> usize {
    self.bindgroup_indices.len()
  }
  pub fn get_bindgroup(&self, index: usize) -> Index {
    self.bindgroup_indices[index]
  }
}

// pub trait ShadingParamGroup: Any {
//   fn get_gpu_bindgroup(&self) -> &WGPUBindGroup;
// }

pub struct ResourceManager {
  geometries: Arena<Box<dyn Geometry>>,
  shadings: Arena<Shading>,
  bindgroups: Arena<WGPUBindGroup>,
}

impl ResourceManager {
  pub fn new() -> Self {
    Self {
      geometries: Arena::new(),
      shadings: Arena::new(),
      bindgroups: Arena::new(),
    }
  }

  pub fn add_geometry(&mut self, geometry: impl Geometry) -> Index {
    self.geometries.insert(Box::new(geometry))
  }

  pub fn get_geometry_mut(&mut self, index: Index) -> &mut dyn Geometry {
    self.geometries.get_mut(index).unwrap().as_mut()
  }

  pub fn get_geometry(&self, index: Index) -> &dyn Geometry {
    self.geometries.get(index).unwrap().as_ref()
  }

  pub fn delete_geometry(&mut self, index: Index) {
    self.geometries.remove(index);
  }

  pub fn add_shading(&mut self, shading: Shading) -> Index {
    self.shadings.insert(shading)
  }

  pub fn get_shading_mut(&mut self, index: Index) -> &mut Shading {
    self.shadings.get_mut(index).unwrap()
  }

  pub fn get_shading(&self, index: Index) -> &Shading {
    self.shadings.get(index).unwrap()
  }

  pub fn add_bindgroup(&mut self, shading_params: WGPUBindGroup) -> Index {
    self.bindgroups.insert(shading_params)
  }

  pub fn get_bindgroup_mut(&mut self, index: Index) -> &mut WGPUBindGroup {
    self.bindgroups.get_mut(index).unwrap()
  }

  pub fn get_bindgroup(&self, index: Index) -> &WGPUBindGroup {
    self.bindgroups.get(index).unwrap()
  }
}
