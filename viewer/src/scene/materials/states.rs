use std::{collections::HashMap, rc::Rc, sync::Mutex};

use rendiation_webgpu::{PipelineVariantContainer, PipelineVariantKey};

use crate::scene::{ValueID, ValueIDGenerator};

pub static STATE_ID: once_cell::sync::Lazy<Mutex<ValueIDGenerator<MaterialStates>>> =
  once_cell::sync::Lazy::new(|| Mutex::new(ValueIDGenerator::default()));

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MaterialStates {
  pub depth_write_enabled: bool,
  pub depth_compare: wgpu::CompareFunction,
  // pub stencil: wgpu::StencilState,
  // pub bias: Default::default(),
  pub blend: Option<wgpu::BlendState>,
  pub write_mask: wgpu::ColorWrites,
}

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
      stencil: Default::default(),
      bias: Default::default(),
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
  fn request(&mut self, variant: &Self::Key, creator: impl FnOnce() -> wgpu::RenderPipeline) {
    self
      .pipelines
      .entry(variant.current)
      .or_insert_with(Default::default)
      .request(&variant.inner, creator);
  }

  fn retrieve(&self, variant: &Self::Key) -> &Rc<wgpu::RenderPipeline> {
    self
      .pipelines
      .get(&variant.current)
      .unwrap()
      .retrieve(&variant.inner)
  }
}
