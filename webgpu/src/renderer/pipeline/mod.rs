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
  pool: HashMap<(TargetStates, RasterizationState), WGPUPipeline>, // todo optimize
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

  pub fn get(
    &self,
    target_states: &RenderTargetFormatsInfo,
    renderer: &wgpu::Device,
  ) -> &wgpu::RenderPipeline {
    todo!()
    // let mut builder = self.builder.borrow_mut();
    // let key = (target_states, raster_states);
    // self
    //   .pool
    //   .entry(key) // todo optimize
    //   .or_insert_with(|| {
    //     self.builder.target_states = target_states;
    //   })
  }
}

impl AsMut<Self> for WGPUPipeline {
  fn as_mut(&mut self) -> &mut Self {
    self
  }
}
