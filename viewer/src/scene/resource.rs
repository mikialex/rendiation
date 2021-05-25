use std::any::Any;

use arena::Arena;

pub struct SceneResource {
  // pipeline_cache: Vec<wgpu::RenderPipeline>,
  // bindgroup_cache: Vec<wgpu::BindGroup>,
  pub buffers: Arena<wgpu::Buffer>,
  pub material_gpu: Arena<Box<dyn Any>>,
}

impl SceneResource {
  pub fn new() -> Self {
    Self {
      // pipeline_cache: Vec::new(),
      buffers: Arena::new(),
      material_gpu: Arena::new(),
    }
  }
}
