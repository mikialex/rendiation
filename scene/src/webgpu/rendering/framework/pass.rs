use std::marker::PhantomData;

use rendiation_webgpu::{Operations, RenderPassDescriptorOwned, GPU};

use crate::{Attachment, AttachmentWriteView, FrameCtx, SceneRenderPass};

pub fn pass(name: impl Into<String>) -> PassDescriptor<'static> {
  let mut desc = RenderPassDescriptorOwned::default();
  desc.name = name.into();
  PassDescriptor {
    phantom: PhantomData,
    desc,
  }
}

pub struct PassDescriptor<'a> {
  phantom: PhantomData<&'a Attachment>,
  desc: RenderPassDescriptorOwned,
}

impl<'a> PassDescriptor<'a> {
  #[must_use]
  pub fn with_color(
    mut self,
    attachment: AttachmentWriteView<&'a mut Attachment>,
    op: impl Into<wgpu::Operations<wgpu::Color>>,
  ) -> Self {
    self.desc.channels.push((op.into(), attachment.view));
    self
  }

  #[must_use]
  pub fn with_depth(
    mut self,
    attachment: AttachmentWriteView<&'a mut Attachment>,
    op: impl Into<wgpu::Operations<f32>>,
  ) -> Self {
    self
      .desc
      .depth_stencil_target
      .replace((op.into(), attachment.view));

    // todo check sample count is same as color's

    self
  }

  #[must_use]
  pub fn resolve_to(mut self, attachment: AttachmentWriteView<&'a mut Attachment>) -> Self {
    self.desc.resolve_target = attachment.view.into();
    self
  }

  #[must_use]
  pub fn render<'x>(self, ctx: &'x mut FrameCtx) -> ActiveRenderPass<'x> {
    let pass = ctx.encoder.begin_render_pass(self.desc.clone());

    let pass = SceneRenderPass {
      pass,
      binding: Default::default(),
      resources: ctx.resources,
    };

    ActiveRenderPass {
      desc: self.desc,
      gpu: ctx.gpu,
      pass,
    }
  }
}

pub trait PassContent {
  fn render(&mut self, gpu: &GPU, pass: &mut SceneRenderPass);
}

impl<T: PassContent> PassContent for Option<T> {
  fn render(&mut self, gpu: &GPU, pass: &mut SceneRenderPass) {
    if let Some(content) = self {
      content.render(gpu, pass);
    }
  }
}

pub struct ActiveRenderPass<'p> {
  pass: SceneRenderPass<'p, 'p>,
  gpu: &'p GPU,
  pub desc: RenderPassDescriptorOwned,
}

impl<'p> ActiveRenderPass<'p> {
  #[allow(clippy::return_self_not_must_use)]
  pub fn by(mut self, renderable: &mut dyn PassContent) -> Self {
    renderable.render(self.gpu, &mut self.pass);
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
