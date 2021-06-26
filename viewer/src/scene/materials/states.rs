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

pub struct StatePipelineVariant<T> {
  pipelines: HashMap<ValueID<PreferredMaterialStates>, T>,
}

impl<T> Default for StatePipelineVariant<T> {
  fn default() -> Self {
    Self {
      pipelines: Default::default(),
    }
  }
}

impl AsRef<ValueID<PreferredMaterialStates>> for ValueID<PreferredMaterialStates> {
  fn as_ref(&self) -> &ValueID<PreferredMaterialStates> {
    self
  }
}

pub trait PipelineVariantContainer<V>: Default {
  fn request(&mut self, variant: &V, creator: impl FnOnce() -> wgpu::RenderPipeline);

  fn retrieve(&self, variant: &V) -> &wgpu::RenderPipeline;
}

pub enum PipelineUnit {
  Created(wgpu::RenderPipeline),
  Empty,
}
impl Default for PipelineUnit {
  fn default() -> Self {
    PipelineUnit::Empty
  }
}

impl<V> PipelineVariantContainer<V> for PipelineUnit {
  fn request(&mut self, variant: &V, creator: impl FnOnce() -> wgpu::RenderPipeline) {
    *self = PipelineUnit::Created(creator());
  }
  fn retrieve(&self, variant: &V) -> &wgpu::RenderPipeline {
    match self {
      PipelineUnit::Created(p) => p,
      PipelineUnit::Empty => unreachable!(),
    }
  }
}

impl<T, V> PipelineVariantContainer<V> for StatePipelineVariant<T>
where
  T: PipelineVariantContainer<V>,
  V: AsRef<ValueID<PreferredMaterialStates>>,
{
  fn request(&mut self, variant: &V, creator: impl FnOnce() -> wgpu::RenderPipeline) {
    self
      .pipelines
      .entry(*variant.as_ref())
      .or_insert_with(Default::default)
      .request(variant, creator);
  }

  fn retrieve(&self, variant: &V) -> &wgpu::RenderPipeline {
    self
      .pipelines
      .get(variant.as_ref())
      .unwrap()
      .retrieve(variant)
  }
}

pub struct TopologyPipelineVariant<T> {
  pipelines: [Option<T>; 5],
}

impl<T> Default for TopologyPipelineVariant<T> {
  fn default() -> Self {
    todo!()
  }
}

impl<T, V> PipelineVariantContainer<V> for TopologyPipelineVariant<T>
where
  T: PipelineVariantContainer<V>,
  V: AsRef<wgpu::PrimitiveTopology>,
{
  fn request(&mut self, variant: &V, creator: impl FnOnce() -> wgpu::RenderPipeline) {
    let index = *variant.as_ref() as usize;
    self.pipelines[index]
      .get_or_insert_with(Default::default)
      .request(variant, creator);
  }

  fn retrieve(&self, variant: &V) -> &wgpu::RenderPipeline {
    let index = *variant.as_ref() as usize;
    self.pipelines[index].as_ref().unwrap().retrieve(variant)
  }
}
