use std::{
  cell::RefCell,
  marker::PhantomData,
  ops::{Deref, DerefMut},
};

use rendiation_webgpu::{
  GPURenderPass, Operations, RenderPassDescriptorOwned, RenderPassInfo, GPU,
};

use crate::{Attachment, AttachmentWriteView, PassGPUDataCache, RenderEngine, Scene};

pub fn pass<'t>(name: impl Into<String>) -> PassDescriptor<'static, 't> {
  let mut desc = RenderPassDescriptorOwned::default();
  desc.name = name.into();
  PassDescriptor {
    phantom: PhantomData,
    tasks: Vec::new(),
    desc,
  }
}

pub struct PassUpdateCtx<'a> {
  pub pass_info: &'a RenderPassInfo,
  pub pass_gpu_cache: &'a mut PassGPUDataCache,
}

pub struct SceneRenderPass<'a> {
  pass: GPURenderPass<'a>,
  pub pass_gpu_cache: &'a RefCell<PassGPUDataCache>,
}

impl<'a> Deref for SceneRenderPass<'a> {
  type Target = GPURenderPass<'a>;

  fn deref(&self) -> &Self::Target {
    &self.pass
  }
}

impl<'a> DerefMut for SceneRenderPass<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.pass
  }
}

pub trait PassContent {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, ctx: &PassUpdateCtx);
  fn setup_pass<'a>(&'a self, pass: &mut SceneRenderPass<'a>, scene: &'a Scene);
}

impl<T: PassContent> PassContent for Option<T> {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, pass_info: &PassUpdateCtx) {
    if let Some(c) = self {
      c.update(gpu, scene, pass_info);
    }
  }

  fn setup_pass<'a>(&'a self, pass: &mut SceneRenderPass<'a>, scene: &'a Scene) {
    if let Some(c) = self {
      c.setup_pass(pass, scene);
    }
  }
}

pub struct PassDescriptor<'a, 't> {
  phantom: PhantomData<&'a Attachment<wgpu::TextureFormat>>,
  tasks: Vec<&'t mut dyn PassContent>,

  desc: RenderPassDescriptorOwned,
}

impl<'a, 't> PassDescriptor<'a, 't> {
  #[must_use]
  pub fn with_color(
    mut self,
    attachment: AttachmentWriteView<'a, wgpu::TextureFormat>,
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
    attachment: AttachmentWriteView<wgpu::TextureFormat>,
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
  pub fn resolve_to(mut self, attachment: AttachmentWriteView<wgpu::TextureFormat>) -> Self {
    self.desc.resolve_target = attachment.view.into();
    self
  }

  #[must_use]
  pub fn render_by(mut self, renderable: &'t mut dyn PassContent) -> Self {
    self.tasks.push(renderable);
    self
  }

  pub fn render(&mut self, renderable: &'t mut dyn PassContent) -> &mut Self {
    self.tasks.push(renderable);
    self
  }

  pub fn run(mut self, engine: &RenderEngine, scene: &mut Scene) {
    let mut encoder = engine.gpu.encoder.borrow_mut();

    let info = RenderPassInfo {
      buffer_size: self.desc.channels.first().unwrap().2,
      format_info: self.desc.info.clone(),
    };

    {
      let mut pass_cache = engine.pass_cache.borrow_mut();

      let ctx = PassUpdateCtx {
        pass_info: &info,
        pass_gpu_cache: &mut pass_cache,
      };

      for task in &mut self.tasks {
        task.update(&engine.gpu, scene, &ctx)
      }
    }

    let mut pass = encoder.begin_render_pass(&self.desc);

    let camera = scene.active_camera.as_ref().unwrap();
    camera.bounds.setup_viewport(&mut pass);

    let mut pass = SceneRenderPass {
      pass,
      pass_gpu_cache: &engine.pass_cache,
    };

    for task in &self.tasks {
      let pass_index = 0;

      let mut pass_cache = engine.pass_cache.borrow_mut();
      let default_pass_gpu = pass_cache.get_updated_pass_gpu_info(pass_index, &info, &engine.gpu);
      pass.set_bind_group_owned(3, &default_pass_gpu.bindgroup, &[]);

      task.setup_pass(&mut pass, scene)
    }
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
