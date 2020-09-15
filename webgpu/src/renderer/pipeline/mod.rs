use std::collections::HashMap;
use builder::PipelineBuilder;
use rendiation_ral::RasterizationState;

use crate::{TargetStates, WGPURenderer};

mod builder;
mod interface;
pub use builder::*;
pub use interface::*;

pub struct WGPUPipeline {
    pool: HashMap<(TargetStates, wgpu::RasterizationStateDescriptor), WGPUPipeline>, // todo optimize
    builder: PipelineBuilder,
  }
  
  impl WGPUPipeline {
    pub fn new(
      vertex_shader: Vec<u32>,
      frag_shader: Vec<u32>,
      shader_interface_info: PipelineShaderInterfaceInfo,
    ) -> Self {
      Self {
        pool: HashMap::new(),
        builder: PipelineBuilder::new(vertex_shader, frag_shader, shader_interface_info),
      }
    }
  
    pub fn clear(&mut self) {
      self.pool.clear()
    }
  
    pub fn get(
      &mut self,
      target_states: &TargetStates,
      raster_states: &RasterizationState,
      renderer: &WGPURenderer,
    ) -> WGPUPipeline {
      todo!()
      // let key = (target_states, raster_states);
      // self
      //   .pool
      //   .entry(key) // todo optimize
      //   .or_insert_with(|| {
      //     self.builder.target_states = target_states;
      //   })
    }
  }
  