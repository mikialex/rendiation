use std::{
  cell::RefCell,
  marker::PhantomData,
  ops::{Deref, DerefMut},
};

use rendiation_webgpu::{
  GPUCommandEncoder, GPURenderPass, Operations, RenderPassDescriptorOwned, RenderPassInfo, GPU,
};

use crate::{Attachment, AttachmentWriteView, PassGPUDataCache, RenderEngine, Scene};

pub fn pass(name: impl Into<String>, engine: &RenderEngine) -> PassDescriptor<'static> {
  let mut desc = RenderPassDescriptorOwned::default();
  desc.name = name.into();
  PassDescriptor {
    phantom: PhantomData,
    desc,
  }
}

pub struct PassUpdateCtx<'a> {
  pub pass_info: &'a RenderPassInfo,
  pub pass_gpu_cache: &'a mut PassGPUDataCache,
}

pub struct PassDescriptor<'a> {
  phantom: PhantomData<&'a Attachment>,
  desc: RenderPassDescriptorOwned,
}

impl<'a> PassDescriptor<'a> {
  #[must_use]
  pub fn with_color(
    mut self,
    attachment: AttachmentWriteView<'a>,
    op: impl Into<wgpu::Operations<wgpu::Color>>,
  ) -> Self {
    self
      .desc
      .channels
      .push((op.into(), attachment.view, attachment.size));
    self.desc.info.color_formats.push(attachment.format);
    self.desc.info.sample_count = attachment.sample_count;
    self
  }

  #[must_use]
  pub fn with_depth(
    mut self,
    attachment: AttachmentWriteView,
    op: impl Into<wgpu::Operations<f32>>,
  ) -> Self {
    self
      .desc
      .depth_stencil_target
      .replace((op.into(), attachment.view));

    self
      .desc
      .info
      .depth_stencil_format
      .replace(attachment.format);

    self.desc.info.sample_count = attachment.sample_count;
    // todo check sample count is same as color's

    self
  }

  #[must_use]
  pub fn resolve_to(mut self, attachment: AttachmentWriteView) -> Self {
    self.desc.resolve_target = attachment.view.into();
    self
  }

  pub fn run<'e>(
    mut self,
    engine: &RenderEngine,
    encoder: &'e mut GPUCommandEncoder,
  ) -> ActiveRenderPass<'e> {
    let info = RenderPassInfo {
      buffer_size: self.desc.channels.first().unwrap().2,
      format_info: self.desc.info.clone(),
    };

    #[cfg(all(target_arch = "wasm32", feature = "webgl"))]
    if let Some(resolve_target) = self.desc.resolve_target.take() {
      self.desc.channels[0].1 = resolve_target
    }

    ActiveRenderPass {
      desc: self.desc,
      pass: encoder.begin_render_pass(&self.desc),
    }
  }
}

pub trait PassContent {
  fn render(&self, pass: &mut GPURenderPass);
}

pub struct ActiveRenderPass<'p> {
  desc: RenderPassDescriptorOwned,
  pass: GPURenderPass<'p>,
}

impl<'p> ActiveRenderPass<'p> {
  #[must_use]
  pub fn render(mut self, renderable: &mut dyn PassContent) -> Self {
    renderable.render(&mut self.pass);
    self
  }
}

pub fn color(r: f64, g: f64, b: f64) -> wgpu::Color {
  wgpu::Color { r, g, b, a: 1. }
}

pub fn all_zero() -> wgpu::Color {
  wgpu::Color {
    r: 0.,
    g: 0.,
    b: 0.,
    a: 0.,
  }
}

pub fn color_same(r: f64) -> wgpu::Color {
  wgpu::Color {
    r,
    g: r,
    b: r,
    a: 1.,
  }
}

pub fn clear<V>(v: V) -> Operations<V> {
  wgpu::Operations {
    load: wgpu::LoadOp::Clear(v),
    store: true,
  }
}

pub fn load<V>() -> Operations<V> {
  wgpu::Operations {
    load: wgpu::LoadOp::Load,
    store: true,
  }
}
