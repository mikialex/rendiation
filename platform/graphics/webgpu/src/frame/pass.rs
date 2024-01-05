use __core::marker::PhantomData;
use rendiation_algebra::*;
use rendiation_shader_api::{std140_layout, ShaderStruct};

use crate::*;

pub struct FrameRenderPass<'encoder, 'b> {
  pub ctx: GPURenderPassCtx<'encoder, 'b>,
  pub pass_info: UniformBufferDataView<RenderPassGPUInfoData>,
}

impl<'a, 'b> std::ops::Deref for FrameRenderPass<'a, 'b> {
  type Target = GPURenderPass<'a>;

  fn deref(&self) -> &Self::Target {
    &self.ctx.pass
  }
}

impl<'a, 'b> std::ops::DerefMut for FrameRenderPass<'a, 'b> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.ctx.pass
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, PartialEq, ShaderStruct)]
pub struct RenderPassGPUInfoData {
  pub texel_size: Vec2<f32>,
  pub buffer_size: Vec2<f32>,
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

  fn buffer_size(&self) -> Vec2<f32> {
    self
      .desc
      .channels
      .first()
      .map(|c| &c.1)
      .or_else(|| self.desc.depth_stencil_target.as_ref().map(|c| &c.1))
      .map(|c| Vec2::from(c.size().into_usize()).map(|v| v as f32))
      .unwrap_or_else(Vec2::zero)
  }

  #[must_use]
  pub fn resolve_to(mut self, attachment: AttachmentView<&'a mut Attachment>) -> Self {
    self.desc.resolve_target = attachment.view.into();
    self
  }

  #[must_use]
  pub fn render_ctx<'x>(self, ctx: &'x mut FrameCtx) -> ActiveRenderPass<'x> {
    self.render(&mut ctx.encoder, ctx.gpu)
  }

  #[must_use]
  pub fn render<'x>(
    self,
    encoder: &'x mut GPUCommandEncoder,
    gpu: &'x GPU,
  ) -> ActiveRenderPass<'x> {
    let pass = encoder.begin_render_pass(self.desc.clone());

    let buffer_size = self.buffer_size();
    let pass_info = RenderPassGPUInfoData {
      texel_size: buffer_size.map(|v| 1.0 / v),
      buffer_size,
      ..Zeroable::zeroed()
    };
    let pass_info = create_uniform(pass_info, gpu);

    let c = GPURenderPassCtx::new(pass, gpu);

    let pass = FrameRenderPass { ctx: c, pass_info };

    ActiveRenderPass {
      desc: self.desc,
      pass,
    }
  }
}

pub trait PassContent {
  fn render(&mut self, pass: &mut FrameRenderPass);
}

impl<T: PassContent> PassContent for Option<T> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    if let Some(content) = self {
      content.render(pass);
    }
  }
}

pub struct ActiveRenderPass<'p> {
  pass: FrameRenderPass<'p, 'p>,
  pub desc: RenderPassDescriptorOwned,
}

impl<'p> ActiveRenderPass<'p> {
  pub fn by(mut self, mut renderable: impl PassContent) -> Self {
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
    store: true,
  }
}

pub fn load<V>() -> gpu::Operations<V> {
  gpu::Operations {
    load: gpu::LoadOp::Load,
    store: true,
  }
}
