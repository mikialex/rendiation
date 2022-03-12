use std::marker::PhantomData;

use rendiation_webgpu::{GPURenderPass, Operations, RenderPassDescriptorOwned, GPU};

use crate::{Attachment, AttachmentWriteView, RenderEngine, SceneRenderPass};

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
    attachment: AttachmentWriteView<'a>,
    op: impl Into<wgpu::Operations<wgpu::Color>>,
  ) -> Self {
    self.desc.channels.push((op.into(), attachment.view));
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

    // todo check sample count is same as color's

    self
  }

  #[must_use]
  pub fn resolve_to(mut self, attachment: AttachmentWriteView) -> Self {
    self.desc.resolve_target = attachment.view.into();
    self
  }

  #[must_use]
  pub fn run(self, engine: &mut RenderEngine) -> ActiveRenderPass {
    #[cfg(all(target_arch = "wasm32", feature = "webgl"))]
    if let Some(resolve_target) = self.desc.resolve_target.take() {
      self.desc.channels[0].1 = resolve_target
    }

    let pass = engine.encoder.begin_render_pass(&self.desc);

    // safety: the pass will reference the desc which refs the texture views to write
    // and they drop together.
    // todo, move the desc into command encoder and we can remove this unsafe
    let pass = unsafe { std::mem::transmute(pass) };

    ActiveRenderPass {
      desc: self.desc,
      gpu: &engine.gpu,
      pass,
    }
  }
}

pub trait PassContent {
  fn render(&mut self, gpu: &GPU, pass: &mut SceneRenderPass);
}

pub struct ActiveRenderPass<'p> {
  pass: GPURenderPass<'p>,
  gpu: &'p GPU,
  pub desc: RenderPassDescriptorOwned,
}

impl<'p> ActiveRenderPass<'p> {
  #[must_use]
  pub fn render(mut self, renderable: &mut dyn PassContent) -> Self {
    let pass = SceneRenderPass {
      pass: &mut self.pass,
      binding: Default::default(),
      resources: todo!(),
    };
    renderable.render(self.gpu, &mut pass);
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
