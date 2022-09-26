use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, PartialEq, ShaderStruct)]
pub struct RenderPassGPUInfoData {
  pub texel_size: Vec2<f32>,
  pub buffer_size: Vec2<f32>,
}

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
    op: impl Into<webgpu::Operations<webgpu::Color>>,
  ) -> Self {
    self.desc.channels.push((op.into(), attachment.into()));
    self
  }

  #[must_use]
  pub fn with_depth(
    mut self,
    attachment: impl Into<RenderTargetView> + 'a,
    op: impl Into<webgpu::Operations<f32>>,
  ) -> Self {
    self
      .desc
      .depth_stencil_target
      .replace((op.into(), attachment.into()));

    // todo check sample count is same as color's

    self
  }

  fn buffer_size(&self) -> Vec2<f32> {
    let first = &self.desc.channels.first().unwrap().1;
    let size = first.size().into_usize();
    Vec2::new(size.0 as f32, size.1 as f32)
  }

  #[must_use]
  pub fn resolve_to(mut self, attachment: AttachmentView<&'a mut Attachment>) -> Self {
    self.desc.resolve_target = attachment.view.into();
    self
  }

  #[must_use]
  pub fn render<'x>(self, ctx: &'x mut FrameCtx) -> ActiveRenderPass<'x> {
    let pass = ctx.encoder.begin_render_pass(self.desc.clone());

    let buffer_size = self.buffer_size();
    let pass_info = RenderPassGPUInfoData {
      texel_size: buffer_size.map(|v| 1.0 / v),
      buffer_size,
      ..Zeroable::zeroed()
    };
    let pass_info = create_uniform(pass_info, ctx.gpu);

    let c = GPURenderPassCtx::new(pass, ctx.gpu);

    let pass = SceneRenderPass {
      ctx: c,
      resources: ctx.resources,
      pass_info,
    };

    ActiveRenderPass {
      desc: self.desc,
      pass,
    }
  }
}

pub trait PassContent {
  fn render(&mut self, pass: &mut SceneRenderPass);
}

impl<T: PassContent> PassContent for Option<T> {
  fn render(&mut self, pass: &mut SceneRenderPass) {
    if let Some(content) = self {
      content.render(pass);
    }
  }
}

pub struct ActiveRenderPass<'p> {
  pass: SceneRenderPass<'p, 'p, 'p>,
  pub desc: RenderPassDescriptorOwned,
}

impl<'p> ActiveRenderPass<'p> {
  #[allow(clippy::return_self_not_must_use)]
  pub fn by(mut self, mut renderable: impl PassContent) -> Self {
    renderable.render(&mut self.pass);
    self
  }
}

pub fn color(r: f64, g: f64, b: f64) -> webgpu::Color {
  webgpu::Color { r, g, b, a: 1. }
}

pub fn all_zero() -> webgpu::Color {
  webgpu::Color {
    r: 0.,
    g: 0.,
    b: 0.,
    a: 0.,
  }
}

pub fn color_same(r: f64) -> webgpu::Color {
  webgpu::Color {
    r,
    g: r,
    b: r,
    a: 1.,
  }
}

pub fn clear<V>(v: V) -> Operations<V> {
  webgpu::Operations {
    load: webgpu::LoadOp::Clear(v),
    store: true,
  }
}

pub fn load<V>() -> Operations<V> {
  webgpu::Operations {
    load: webgpu::LoadOp::Load,
    store: true,
  }
}
