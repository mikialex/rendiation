use std::{collections::HashMap, rc::Rc, sync::Mutex};

use rendiation_webgpu::{PipelineVariantContainer, PipelineVariantKey};

use crate::scene::{ValueID, ValueIDGenerator};

pub static STATE_ID: once_cell::sync::Lazy<Mutex<ValueIDGenerator<MaterialStates>>> =
  once_cell::sync::Lazy::new(|| Mutex::new(ValueIDGenerator::default()));

#[derive(Debug, Clone, PartialEq)]
pub struct MaterialStates {
  pub depth_write_enabled: bool,
  pub depth_compare: wgpu::CompareFunction,
  pub stencil: wgpu::StencilState,
  pub bias: wgpu::DepthBiasState,
  pub blend: Option<wgpu::BlendState>,
  pub write_mask: wgpu::ColorWrites,
}

impl std::hash::Hash for MaterialStates {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.depth_write_enabled.hash(state);
    self.depth_compare.hash(state);
    self.stencil.hash(state);
    self.bias.slope_scale.to_bits().hash(state);
    self.bias.clamp.to_bits().hash(state);
    self.bias.constant.hash(state);
    self.blend.hash(state);
    self.write_mask.hash(state);
  }
}

impl Eq for MaterialStates {}

impl MaterialStates {
  pub fn map_color_states(&self, format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
      format,
      blend: self.blend,
      write_mask: self.write_mask,
    }
  }
  pub fn map_depth_stencil_state(
    &self,
    format: Option<wgpu::TextureFormat>,
  ) -> Option<wgpu::DepthStencilState> {
    format.map(|format| wgpu::DepthStencilState {
      format,
      depth_write_enabled: self.depth_write_enabled,
      depth_compare: self.depth_compare,
      stencil: self.stencil.clone(),
      bias: self.bias,
    })
  }
}

impl Default for MaterialStates {
  fn default() -> Self {
    Self {
      depth_write_enabled: true,
      depth_compare: wgpu::CompareFunction::Less,
      blend: None,
      write_mask: wgpu::ColorWrites::all(),
      bias: Default::default(),
      stencil: Default::default(),
    }
  }
}

pub struct StatePipelineVariant<T> {
  pipelines: HashMap<ValueID<MaterialStates>, T>,
}

impl<T> Default for StatePipelineVariant<T> {
  fn default() -> Self {
    Self {
      pipelines: Default::default(),
    }
  }
}

impl<T: PipelineVariantContainer> PipelineVariantContainer for StatePipelineVariant<T> {
  type Key = PipelineVariantKey<T::Key, ValueID<MaterialStates>>;
  fn request(
    &mut self,
    variant: &Self::Key,
    creator: impl FnOnce() -> wgpu::RenderPipeline,
  ) -> &Rc<wgpu::RenderPipeline> {
    self
      .pipelines
      .entry(variant.current)
      .or_insert_with(Default::default)
      .request(&variant.inner, creator)
  }
}
