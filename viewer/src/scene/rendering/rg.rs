use std::{cell::RefCell, collections::HashMap, rc::Rc};

use rendiation_algebra::Vec3;
use rendiation_texture::Size;
use rendiation_webgpu::*;

use crate::{Scene, StandardForward, ViewerRenderPass};

pub struct ResourcePoolInner {
  pub attachments: HashMap<(Size, wgpu::TextureFormat), Vec<wgpu::Texture>>,
}

#[derive(Clone)]
pub struct ResourcePool {
  pub inner: Rc<RefCell<ResourcePoolInner>>,
}

impl Default for ResourcePool {
  fn default() -> Self {
    todo!()
  }
}

pub struct RenderEngine {
  resource: ResourcePool,
  gpu: GPU,
  output_size: Size,
  output: wgpu::TextureView,
}

impl RenderEngine {
  pub fn screen(&self) -> Attachment<wgpu::TextureFormat> {
    todo!()
  }
}

pub fn attachment() -> AttachmentDescriptor<wgpu::TextureFormat> {
  AttachmentDescriptor {
    format: wgpu::TextureFormat::Rgba8Unorm,
    sizer: default_sizer(),
  }
}

pub fn depth_attachment() -> AttachmentDescriptor<wgpu::TextureFormat> {
  AttachmentDescriptor {
    format: wgpu::TextureFormat::Depth24PlusStencil8,
    sizer: default_sizer(),
  }
}

pub trait AttachmentFormat: Into<wgpu::TextureFormat> + Copy {}
impl<T: Into<wgpu::TextureFormat> + Copy> AttachmentFormat for T {}

#[derive(Clone)]
pub struct Attachment<F: AttachmentFormat> {
  pool: ResourcePool,
  des: AttachmentDescriptor<F>,
  size: Size,
  texture: Option<Rc<wgpu::Texture>>,
}

impl<F: AttachmentFormat> Attachment<F> {
  pub fn write(&mut self) -> AttachmentWriteView<F> {
    todo!()
  }
}

impl<F: AttachmentFormat> Drop for Attachment<F> {
  fn drop(&mut self) {
    if let Ok(texture) = Rc::try_unwrap(self.texture.take().unwrap()) {
      let mut pool = self.pool.inner.borrow_mut();
      let cached = pool
        .attachments
        .entry((self.size, self.des.format.into()))
        .or_insert_with(Default::default);

      cached.push(texture)
    }
  }
}

pub struct AttachmentWriteView<'a, F: AttachmentFormat> {
  attachment: &'a mut Attachment<F>,
  view: wgpu::TextureView,
}

pub struct AttachmentReadView<'a, F: AttachmentFormat> {
  attachment: &'a Attachment<F>,
  view: wgpu::TextureView,
}

pub struct AttachmentDescriptor<F> {
  format: F,
  sizer: Box<dyn Fn(Size) -> Size>,
}

impl<F> Clone for AttachmentDescriptor<F> {
  fn clone(&self) -> Self {
    todo!()
  }
}

fn default_sizer() -> Box<dyn Fn(Size) -> Size> {
  Box::new(|size| size)
}

impl<F: AttachmentFormat> AttachmentDescriptor<F> {
  pub fn format(mut self, format: F) -> Self {
    self.format = format;
    self
  }
}

impl<F: AttachmentFormat> AttachmentDescriptor<F> {
  pub fn request(self, engine: &RenderEngine) -> Attachment<F> {
    let size = (self.sizer)(engine.output_size);
    let mut resource = engine.resource.inner.borrow_mut();
    let cached = resource
      .attachments
      .entry((size, self.format.into()))
      .or_insert_with(Default::default);
    let texture = cached.pop().unwrap_or_else(|| {
      engine.gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: size.into_gpu_size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: self.format.into(),
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
      })
    });
    Attachment {
      pool: engine.resource.clone(),
      des: self,
      size,
      texture: Rc::new(texture).into(),
    }
  }
}

pub trait Pipeline {
  fn render(&mut self, engine: &RenderEngine, scene: &SceneDispatcher);
}

pub struct HighLight {
  color: Vec3<f32>,
}

impl ViewerRenderPass for HighLight {
  fn depth_stencil_format(&self) -> Option<wgpu::TextureFormat> {
    wgpu::TextureFormat::Depth32Float.into()
  }

  fn color_format(&self) -> &[wgpu::TextureFormat] {
    // self.color_format.as_slice()
    todo!()
  }
}

pub struct SimplePipeline {
  forward: StandardForward,
  highlight: HighLight,
}

pub struct SceneDispatcher {
  scene: Rc<RefCell<Scene>>,
}

impl SceneDispatcher {
  pub fn create_content<T>(&self, test: &mut T) -> impl PassContent {
    ForwardPass
  }
}

pub struct ForwardPass;

impl PassContent for ForwardPass {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, resource: &mut ResourcePoolInner) {
    todo!()
  }

  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    scene: &'a Scene,
    resource: &'a ResourcePoolInner,
  ) {
    todo!()
  }
}

pub trait PassContent: 'static {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, resource: &mut ResourcePoolInner);
  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    scene: &'a Scene,
    resource: &'a ResourcePoolInner,
  );
}

impl Pipeline for SimplePipeline {
  #[rustfmt::skip]
  fn render(&mut self, engine: &RenderEngine, scene: &SceneDispatcher) {
    let scene_main_content = scene.create_content(&mut self.forward);

    let mut scene_color = attachment()
      .format(wgpu::TextureFormat::Rgba8Unorm)
      .request(engine);

    let mut scene_depth = depth_attachment()
      .format(wgpu::TextureFormat::Depth32Float)
      .request(engine);

    pass("scene_pass")
      .with_color(scene_color.write(), clear(color(0.1, 0.2, 0.3)))
      .with_depth(scene_depth.write(), clear(1.))
      .render_by(scene_main_content)
      .run(engine, scene);

    let mut high_light_object_mask = attachment()
      .format(wgpu::TextureFormat::Rgba8Unorm)
      .request(engine);

    let high_light_object = scene.create_content(&mut self.highlight);

    pass("high_light_pass")
      .with_color( high_light_object_mask.write(), clear(color_same(1.)))
      .render_by(high_light_object)
      .run(engine, scene);

    pass("final_compose")
      // .with_color(scene_color.write(), clear(color(0.1, 0.2, 0.3))) // read write same texture is compile error
      .with_color(engine.screen().write(), clear(color_same(1.)))
      .render_by(copy(scene_color))
      .render_by(high_light_blend(high_light_object_mask))
      .run(engine, scene);
  }
}

pub struct HiLighter {
  source: Attachment<wgpu::TextureFormat>,
}

impl PassContent for HiLighter {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, resource: &mut ResourcePoolInner) {
    // get resource pool texture and view , update bindgroup
    todo!()
  }

  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    scene: &'a Scene,
    resource: &'a ResourcePoolInner,
  ) {
    todo!()
  }
}

pub fn high_light_blend(source: Attachment<wgpu::TextureFormat>) -> impl PassContent {
  ForwardPass
}

pub struct Copier<'a> {
  source: &'a mut Attachment<wgpu::TextureFormat>,
}

impl<'a> Renderable for Copier<'a> {
  fn setup_pass<'r>(&'r self, pass: &mut wgpu::RenderPass<'r>) {
    todo!()
  }
}

pub fn copy(source: Attachment<wgpu::TextureFormat>) -> impl PassContent {
  ForwardPass
}

pub fn pass(name: &'static str) -> PassDescriptor {
  PassDescriptor {
    name,
    channels: Vec::new(),
    tasks: Vec::new(),
    depth_stencil_target: None,
  }
}

pub struct PassDescriptor<'a> {
  name: &'static str,
  channels: Vec<(
    wgpu::Operations<wgpu::Color>,
    &'a mut Attachment<wgpu::TextureFormat>,
    wgpu::TextureView,
  )>,
  tasks: Vec<Box<dyn PassContent>>,
  depth_stencil_target: Option<(
    wgpu::Operations<f32>,
    Attachment<wgpu::TextureFormat>,
    wgpu::TextureView,
  )>,
}

impl<'a> ViewerRenderPass for PassDescriptor<'a> {
  fn depth_stencil_format(&self) -> Option<wgpu::TextureFormat> {
    todo!()
  }

  fn color_format(&self) -> &[wgpu::TextureFormat] {
    // self.color_format.as_slice()
    todo!()
  }
}

impl<'a> PassDescriptor<'a> {
  #[must_use]
  pub fn with_color(
    mut self,
    attachment: AttachmentWriteView<'a, wgpu::TextureFormat>,
    op: impl Into<wgpu::Operations<wgpu::Color>>,
  ) -> Self {
    self
      .channels
      .push((op.into(), attachment.attachment, attachment.view));
    self
  }

  #[must_use]
  pub fn with_depth(
    mut self,
    attachment: AttachmentWriteView<wgpu::TextureFormat>,
    op: impl Into<wgpu::Operations<f32>>,
  ) -> Self {
    self
      .depth_stencil_target
      .replace((op.into(), attachment.attachment.clone(), attachment.view));
    self
  }

  #[must_use]
  pub fn render_by(mut self, renderable: impl PassContent) -> Self {
    self.tasks.push(Box::new(renderable));
    self
  }

  pub fn run(mut self, engine: &RenderEngine, scene: &SceneDispatcher) {
    let mut resource = engine.resource.inner.borrow_mut();

    let mut encoder = engine.gpu.encoder.borrow_mut();

    let color_attachments: Vec<_> = self
      .channels
      .iter()
      .map(|(ops, _, view)| wgpu::RenderPassColorAttachment {
        view,
        resolve_target: None,
        ops: *ops,
      })
      .collect();

    let depth_stencil_attachment = self.depth_stencil_target.as_ref().map(|(ops, _, view)| {
      wgpu::RenderPassDepthStencilAttachment {
        view,
        depth_ops: (*ops).into(),
        stencil_ops: None,
      }
    });

    let mut scene = scene.scene.borrow_mut();

    for task in &mut self.tasks {
      task.update(&engine.gpu, &mut scene, &mut resource)
    }

    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: self.name.into(),
      color_attachments: color_attachments.as_slice(),
      depth_stencil_attachment,
    });

    for task in &self.tasks {
      task.setup_pass(&mut pass, &scene, &resource)
    }
  }
}

pub fn color(r: f64, g: f64, b: f64) -> wgpu::Color {
  wgpu::Color { r, g, b, a: 1. }
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
