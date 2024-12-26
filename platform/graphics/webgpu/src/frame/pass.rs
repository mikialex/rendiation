use core::marker::PhantomData;

use rendiation_algebra::*;
use rendiation_shader_api::{std140_layout, ShaderStruct};

use crate::*;

pub struct FrameRenderPass {
  pub ctx: GPURenderPassCtx,
  pub pass_info: UniformBufferCachedDataView<RenderPassGPUInfoData>,
}

impl std::ops::Deref for FrameRenderPass {
  type Target = GPURenderPass;

  fn deref(&self) -> &Self::Target {
    &self.ctx.pass
  }
}

impl std::ops::DerefMut for FrameRenderPass {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.ctx.pass
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, PartialEq, ShaderStruct, Default)]
pub struct RenderPassGPUInfoData {
  pub texel_size: Vec2<f32>,
  pub buffer_size: Vec2<f32>,
}

impl RenderPassGPUInfoData {
  pub fn new(texel_size: Vec2<f32>, buffer_size: Vec2<f32>) -> Self {
    Self {
      texel_size,
      buffer_size,
      ..Default::default()
    }
  }
}

/// Create a pass descriptor with given name. The provide name is used for debug purpose, not
/// required to be unique
pub fn pass(name: impl Into<String>) -> PassDescriptor<'static> {
  let desc = RenderPassDescriptorOwned {
    name: name.into(),
    ..Default::default()
  };
  PassDescriptor {
    phantom: PhantomData,
    desc,
  }
}

pub struct PassDescriptor<'a> {
  phantom: PhantomData<&'a Attachment>,
  desc: RenderPassDescriptorOwned,
}

impl<'a> From<AttachmentView<&'a mut Attachment>> for RenderTargetView {
  fn from(val: AttachmentView<&'a mut Attachment>) -> Self {
    val.view
  }
}

impl<'a> PassDescriptor<'a> {
  #[must_use]
  pub fn with_desc(mut self, desc: RenderPassDescriptorOwned) -> Self {
    self.desc = desc;
    self
  }

  #[must_use]
  pub fn with_color(
    mut self,
    attachment: impl Into<RenderTargetView> + 'a,
    op: impl Into<gpu::Operations<gpu::Color>>,
  ) -> Self {
    self.desc.channels.push((op.into(), attachment.into()));
    self
  }

  #[must_use]
  pub fn with_depth(
    mut self,
    attachment: impl Into<RenderTargetView> + 'a,
    op: impl Into<gpu::Operations<f32>>,
  ) -> Self {
    self
      .desc
      .depth_stencil_target
      .replace((op.into(), attachment.into()));

    // todo check sample count is same as color's

    self
  }

  pub fn buffer_size(&self) -> Vec2<f32> {
    self.desc.buffer_size()
  }

  #[must_use]
  pub fn resolve_to(mut self, attachment: AttachmentView<&'a mut Attachment>) -> Self {
    self.desc.resolve_target = attachment.view.into();
    self
  }

  #[must_use]
  pub fn render_ctx(self, ctx: &mut FrameCtx) -> ActiveRenderPass {
    self.render(&mut ctx.encoder, ctx.gpu)
  }

  #[must_use]
  pub fn render(self, encoder: &mut GPUCommandEncoder, gpu: &GPU) -> ActiveRenderPass {
    let pass = encoder.begin_render_pass_with_info(self.desc.clone(), gpu.clone());

    ActiveRenderPass {
      desc: self.desc,
      pass,
    }
  }
}

pub trait PassContent {
  fn render(&mut self, pass: &mut FrameRenderPass);
}
impl<'a> PassContent for Box<dyn PassContent + 'a> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    (**self).render(pass);
  }
}

impl<T: PassContent> PassContent for Option<T> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    if let Some(content) = self {
      content.render(pass);
    }
  }
}

pub struct ActiveRenderPass {
  pub pass: FrameRenderPass,
  pub desc: RenderPassDescriptorOwned,
}

impl ActiveRenderPass {
  pub fn by(mut self, renderable: &mut impl PassContent) -> Self {
    renderable.render(&mut self.pass);
    self
  }
}

pub fn color(r: f64, g: f64, b: f64) -> gpu::Color {
  gpu::Color { r, g, b, a: 1. }
}

pub fn all_zero() -> gpu::Color {
  gpu::Color {
    r: 0.,
    g: 0.,
    b: 0.,
    a: 0.,
  }
}

pub fn color_same(r: f64) -> gpu::Color {
  gpu::Color {
    r,
    g: r,
    b: r,
    a: 1.,
  }
}

pub fn clear<V>(v: V) -> gpu::Operations<V> {
  gpu::Operations {
    load: gpu::LoadOp::Clear(v),
    store: gpu::StoreOp::Store,
  }
}

pub fn load<V>() -> gpu::Operations<V> {
  gpu::Operations {
    load: gpu::LoadOp::Load,
    store: gpu::StoreOp::Store,
  }
}
