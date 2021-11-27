use std::{
  ops::{Deref, DerefMut},
  rc::Rc,
};

#[derive(Clone)]
pub struct RenderPassInfo {
  pub buffer_size: Size,
  pub format_info: PassTargetFormatInfo,
}

#[derive(Clone, Default, Hash)]
pub struct PassTargetFormatInfo {
  pub depth_stencil_format: Option<wgpu::TextureFormat>,
  pub color_formats: Vec<wgpu::TextureFormat>,
}

#[derive(Clone, Default)]
pub struct RenderPassDescriptorOwned {
  pub name: String,
  pub channels: Vec<(wgpu::Operations<wgpu::Color>, Rc<wgpu::TextureView>, Size)>,
  pub depth_stencil_target: Option<(wgpu::Operations<f32>, Rc<wgpu::TextureView>)>,
  pub info: PassTargetFormatInfo,
}

pub struct GPURenderPass<'a> {
  pub(crate) info: RenderPassInfo,
  pub(crate) pass: wgpu::RenderPass<'a>,
  pub(crate) holder: &'a GPURenderPassDataHolder,
}

impl<'a> Deref for GPURenderPass<'a> {
  type Target = wgpu::RenderPass<'a>;

  fn deref(&self) -> &Self::Target {
    &self.pass
  }
}

impl<'a> DerefMut for GPURenderPass<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.pass
  }
}

use rendiation_texture_types::Size;
use typed_arena::Arena;

#[derive(Default)]
pub struct GPURenderPassDataHolder {
  buffers: Arena<Rc<wgpu::Buffer>>,
  bindgroups: Arena<Rc<wgpu::BindGroup>>,
  pipelines: Arena<Rc<wgpu::RenderPipeline>>,
}

impl<'a> GPURenderPass<'a> {
  pub fn info(&self) -> &RenderPassInfo {
    &self.info
  }

  pub fn set_pipeline_owned(&mut self, pipeline: &Rc<wgpu::RenderPipeline>) {
    let pipeline = self.holder.pipelines.alloc(pipeline.clone());
    self.pass.set_pipeline(pipeline)
  }

  pub fn set_bind_group_owned(
    &mut self,
    index: u32,
    bind_group: &Rc<wgpu::BindGroup>,
    offsets: &[wgpu::DynamicOffset],
  ) {
    let bind_group = self.holder.bindgroups.alloc(bind_group.clone());
    self.set_bind_group(index, bind_group, offsets)
  }

  pub fn set_vertex_buffer_owned(&mut self, slot: u32, buffer: &Rc<wgpu::Buffer>) {
    let buffer = self.holder.buffers.alloc(buffer.clone());
    self.pass.set_vertex_buffer(slot, buffer.slice(..))
  }

  pub fn set_index_buffer_owned(
    &mut self,
    buffer: &Rc<wgpu::Buffer>,
    index_format: wgpu::IndexFormat,
  ) {
    let buffer = self.holder.buffers.alloc(buffer.clone());
    self.pass.set_index_buffer(buffer.slice(..), index_format)
  }
}
