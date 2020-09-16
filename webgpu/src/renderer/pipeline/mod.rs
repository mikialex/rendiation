use builder::PipelineBuilder;
use rendiation_ral::RasterizationState;
use std::{cell::RefCell, collections::HashMap};

use crate::{RenderTargetFormatsInfo, TargetStates};

pub mod builder;
pub mod interface;
pub use builder::*;
pub use interface::*;

pub struct WGPUPipeline {
  builder: RefCell<PipelineCacheBuilder>,
  pub rasterization_state: RasterizationState,
}

struct PipelineCacheBuilder {
  pool: HashMap<(TargetStates, RasterizationState), wgpu::RenderPipeline>, // todo optimize
  builder: PipelineBuilder,
}

impl WGPUPipeline {
  pub fn new(
    vertex_shader: Vec<u32>,
    frag_shader: Vec<u32>,
    shader_interface_info: PipelineShaderInterfaceInfo,
  ) -> Self {
    Self {
      builder: RefCell::new(PipelineCacheBuilder {
        pool: HashMap::new(),
        builder: PipelineBuilder::new(vertex_shader, frag_shader, shader_interface_info),
      }),
      rasterization_state: RasterizationState::default(),
    }
  }

  pub fn clear(&self) {
    let mut builder = self.builder.borrow_mut();
    builder.pool.clear();
  }

  // todo optimize
  pub fn get(
    &self,
    _formats_info: &RenderTargetFormatsInfo,
    renderer: &wgpu::Device,
    getter: &mut impl FnMut(&wgpu::RenderPipeline),
  ) {
    let mut builder = self.builder.borrow_mut();
    let target_states = builder
      .builder
      .shader_interface_info
      .preferred_target_states
      .clone();
    let pool = &mut builder.pool;
    // let pipeline_builder = &mut builder.builder;
    let key = (target_states.clone(), self.rasterization_state);
    let pipeline = pool.entry(key).or_insert_with(|| {
      builder.builder.target_states = target_states; // todo merge income states safely

      // pipeline_builder.rasterization = self.rasterization_state; // todo
      builder.builder.build(renderer)
    });
    getter(pipeline);
  }
}

impl AsMut<Self> for WGPUPipeline {
  fn as_mut(&mut self) -> &mut Self {
    self
  }
}
