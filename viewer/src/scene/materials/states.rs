use std::{collections::HashMap, sync::Mutex};

use crate::scene::{ValueID, ValueIDGenerator};

pub static STATE_ID: once_cell::sync::Lazy<Mutex<ValueIDGenerator<PreferredMaterialStates>>> =
  once_cell::sync::Lazy::new(|| Mutex::new(ValueIDGenerator::default()));

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct PreferredMaterialStates {
  pub depth_write_enabled: bool,
  pub depth_compare: wgpu::CompareFunction,
  // pub stencil: wgpu::StencilState,
  // pub bias: Default::default(),
  pub blend: Option<wgpu::BlendState>,
  pub write_mask: wgpu::ColorWrite,
}

impl PreferredMaterialStates {
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

impl Default for PreferredMaterialStates {
  fn default() -> Self {
    Self {
      depth_write_enabled: true,
      depth_compare: wgpu::CompareFunction::Less,
      blend: None,
      write_mask: wgpu::ColorWrite::all(),
    }
  }
}

pub struct StatePipelineVariant {
  pipelines: HashMap<ValueID<PreferredMaterialStates>, wgpu::RenderPipeline>,
}

impl Default for StatePipelineVariant {
  fn default() -> Self {
    Self {
      pipelines: Default::default(),
    }
  }
}

impl StatePipelineVariant {
  pub fn request(
    &mut self,
    uuid: ValueID<PreferredMaterialStates>,
    creator: impl FnOnce() -> wgpu::RenderPipeline,
  ) {
    self.pipelines.entry(uuid).or_insert_with(creator);
  }

  pub fn retrieve(&self, uuid: ValueID<PreferredMaterialStates>) -> &wgpu::RenderPipeline {
    self.pipelines.get(&uuid).unwrap()
  }
}
