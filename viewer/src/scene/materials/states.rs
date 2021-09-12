use std::{collections::HashMap, sync::Mutex};

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
  fn request(&mut self, _variant: &V, creator: impl FnOnce() -> wgpu::RenderPipeline) {
    match self {
      PipelineUnit::Empty => {
        *self = PipelineUnit::Created(creator());
      }
      _ => {}
    }
  }
  fn retrieve(&self, _variant: &V) -> &wgpu::RenderPipeline {
    match self {
      PipelineUnit::Created(p) => p,
      PipelineUnit::Empty => unreachable!(),
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
    Self {
      pipelines: [None, None, None, None, None],
    }
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
