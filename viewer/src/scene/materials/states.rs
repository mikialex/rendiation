use std::{collections::HashMap, rc::Rc, sync::Mutex};

use rendiation_webgpu::PipelineVariantContainer;

use crate::scene::{ValueID, ValueIDGenerator};

pub static STATE_ID: once_cell::sync::Lazy<Mutex<ValueIDGenerator<MaterialStates>>> =
  once_cell::sync::Lazy::new(|| Mutex::new(ValueIDGenerator::default()));

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
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

impl<T, V> PipelineVariantContainer<V> for StatePipelineVariant<T>
where
  T: PipelineVariantContainer<V>,
  V: AsRef<ValueID<MaterialStates>>,
{
  fn request(&mut self, variant: &V, creator: impl FnOnce() -> wgpu::RenderPipeline) {
    self
      .pipelines
      .entry(*variant.as_ref())
      .or_insert_with(Default::default)
      .request(variant, creator);
  }

  fn retrieve(&self, variant: &V) -> &Rc<wgpu::RenderPipeline> {
    self
      .pipelines
      .get(variant.as_ref())
      .unwrap()
      .retrieve(variant)
  }
}
