use builder::PipelineBuilder;
use rendiation_ral::RasterizationState;
use std::{cell::UnsafeCell, collections::HashMap};

use crate::{RenderTargetFormatsInfo, TargetStates};

pub mod builder;
pub mod interface;
pub use builder::*;
pub use interface::*;

pub struct WGPUPipeline {
  builder: UnsafeCell<PipelineCacheBuilder>,
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
      builder: UnsafeCell::new(PipelineCacheBuilder {
        pool: HashMap::new(),
        builder: PipelineBuilder::new(vertex_shader, frag_shader, shader_interface_info),
      }),
      rasterization_state: RasterizationState::default(),
    }
  }

  pub fn clear(&self) {
    let builder = unsafe { &mut *self.builder.get() };
    builder.pool.clear();
  }

  // todo optimize
  pub fn get(
    &self,
    formats_info: &RenderTargetFormatsInfo,
    renderer: &wgpu::Device,
  ) -> &wgpu::RenderPipeline {
    let builder = unsafe { &mut *self.builder.get() };

    let target_states = merge_state(
      &builder
        .builder
        .shader_interface_info
        .preferred_target_states,
      formats_info,
    );

    let pool = &mut builder.pool;
    let pipeline_builder = &mut builder.builder;

    let key = (target_states.clone(), self.rasterization_state);

    pool.entry(key).or_insert_with(|| {
      pipeline_builder.target_states = target_states;

      // pipeline_builder.rasterization = self.rasterization_state; // todo
      pipeline_builder.build(renderer)
    })
  }
}

fn merge_state(preferred: &TargetStates, input: &RenderTargetFormatsInfo) -> TargetStates {
  let mut result = preferred.clone();
  input.depth.as_ref().map(|d| {
    if let Some(result_depth) = &mut result.depth_state {
      result_depth.format = *d;
    } else {
      result.depth_state = Some(wgpu::DepthStencilStateDescriptor {
        format: *d,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::LessEqual,
        stencil: wgpu::StencilStateDescriptor::default(),
      });
    }
  });
  if input.depth.is_none() {
    result.depth_state = None;
  }

  // todo improve
  result.color_states[0].format = input.color[0];

  result
}

impl AsMut<Self> for WGPUPipeline {
  fn as_mut(&mut self) -> &mut Self {
    self
  }
}
