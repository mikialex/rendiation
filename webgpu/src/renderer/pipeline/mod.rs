use builder::PipelineBuilder;
use rendiation_ral::RasterizationStateDescriptor;
use std::{cell::UnsafeCell, collections::HashMap, hash::Hash, hash::Hasher};

use crate::{RenderTargetFormatsInfo, TargetStates};

pub mod builder;
pub mod interface;
pub use builder::*;
pub use interface::*;

#[derive(Default, Debug, Clone)]
pub struct HashAbleRasterizationStateDescriptor {
  desc: RasterizationStateDescriptor,
}

impl Hash for HashAbleRasterizationStateDescriptor {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.desc.front_face.hash(state);
    self.desc.depth_bias.hash(state);
    self.desc.cull_mode.hash(state);
    // todo unsafe float hash
  }
}

impl PartialEq for HashAbleRasterizationStateDescriptor {
  fn eq(&self, other: &Self) -> bool {
    self.desc.front_face.eq(&other.desc.front_face)
      && self.desc.depth_bias.eq(&other.desc.depth_bias)
      && self.desc.cull_mode.eq(&other.desc.cull_mode)
  }
}

impl Eq for HashAbleRasterizationStateDescriptor {}

pub struct WGPUPipeline {
  builder: UnsafeCell<PipelineCacheBuilder>,
  pub rasterization_state: HashAbleRasterizationStateDescriptor,
}

struct PipelineCacheBuilder {
  pool: HashMap<(TargetStates, HashAbleRasterizationStateDescriptor), wgpu::RenderPipeline>, // todo optimize
  builder: PipelineBuilder,
}

pub struct WGPUPipelineBuildSource {
  pub vertex_shader: Vec<u32>,
  pub frag_shader: Vec<u32>,
  pub shader_interface_info: PipelineShaderInterfaceInfo,
}

impl WGPUPipeline {
  pub fn new(source: &WGPUPipelineBuildSource) -> Self {
    Self {
      builder: UnsafeCell::new(PipelineCacheBuilder {
        pool: HashMap::new(),
        builder: PipelineBuilder::new(
          &source.vertex_shader,
          &source.frag_shader,
          source.shader_interface_info.clone(),
        ),
      }),
      rasterization_state: HashAbleRasterizationStateDescriptor::default(),
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

    let key = (target_states.clone(), self.rasterization_state.clone());

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
