use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc};

use rendiation_texture::Size;
use rendiation_webgpu::*;

use crate::*;

#[derive(Default)]
pub struct ResourcePoolInner {
  pub attachments: HashMap<(Size, wgpu::TextureFormat), Vec<wgpu::Texture>>,
}

#[derive(Clone, Default)]
pub struct ResourcePool {
  pub inner: Rc<RefCell<ResourcePoolInner>>,
}

pub struct RenderEngine {
  resource: ResourcePool,
  gpu: GPU,
  output_size: Size,
  output_format: wgpu::TextureFormat,
  output: Rc<wgpu::TextureView>,
}

impl RenderEngine {
  pub fn screen(&self) -> AttachmentWriteView<wgpu::TextureFormat> {
    AttachmentWriteView {
      phantom: PhantomData,
      view: self.output.clone(),
      format: self.output_format,
    }
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
    AttachmentWriteView {
      phantom: PhantomData,
      view: Rc::new(
        self
          .texture
          .as_ref()
          .unwrap()
          .create_view(&wgpu::TextureViewDescriptor::default()),
      ),
      format: self.des.format,
    }
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
  phantom: PhantomData<&'a Attachment<F>>,
  view: Rc<wgpu::TextureView>, // todo opt enum
  format: F,
}

pub struct AttachmentReadView<'a, F: AttachmentFormat> {
  attachment: &'a Attachment<F>,
  view: wgpu::TextureView,
}

#[derive(Clone)]
pub struct AttachmentDescriptor<F> {
  format: F,
  sizer: Rc<dyn Fn(Size) -> Size>,
}

fn default_sizer() -> Rc<dyn Fn(Size) -> Size> {
  Rc::new(|size| size)
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

pub struct SceneDispatcher {
  scene: Rc<RefCell<Scene>>,
}

impl SceneDispatcher {
  pub fn create_content<T>(&self, test: &mut T) -> impl PassContent {
    ForwardScene::default()
  }
}

pub trait PassContent: 'static {
  fn update(
    &mut self,
    gpu: &GPU,
    scene: &mut Scene,
    resource: &mut ResourcePoolInner,
    pass_info: &PassTargetFormatInfo,
  );
  fn setup_pass<'a>(
    &'a self,
    pass: &mut wgpu::RenderPass<'a>,
    scene: &'a Scene,
    resource: &'a ResourcePoolInner,
    pass_info: &'a PassTargetFormatInfo,
  );
}

#[derive(Default)]
pub struct SimplePipeline {
  forward: StandardForward,
  // highlight: HighLight,
}

impl SimplePipeline {
  #[rustfmt::skip]
  pub fn render_simple(&mut self, engine: &RenderEngine, scene: &SceneDispatcher, ) {
    let scene_main_content = scene.create_content(&mut self.forward);

    let mut scene_depth = depth_attachment()
      .format(wgpu::TextureFormat::Depth32Float)
      .request(engine);

    pass("scene_pass")
      .with_color(engine.screen(), scene.scene.borrow().get_main_pass_load_op())
      .with_depth(scene_depth.write(), clear(1.))
      .render_by(BackGroundRendering)
      .render_by(scene_main_content)
      .run(engine, scene);
  }

  // #[rustfmt::skip]
  // pub fn render(&mut self, engine: &RenderEngine, scene: &SceneDispatcher, ) {
  //   let scene_main_content = scene.create_content(&mut self.forward);

  //   let mut scene_color = attachment()
  //     .format(wgpu::TextureFormat::Rgba8Unorm)
  //     .request(engine);

  //   let mut scene_depth = depth_attachment()
  //     .format(wgpu::TextureFormat::Depth32Float)
  //     .request(engine);

  //   pass("scene_pass")
  //     .with_color(scene_color.write(), scene.scene.borrow().get_main_pass_load_op())
  //     .with_depth(scene_depth.write(), clear(1.))
  //     .render_by(scene_main_content)
  //     .run(engine, scene);

  //   let mut high_light_object_mask = attachment()
  //     .format(wgpu::TextureFormat::Rgba8Unorm)
  //     .request(engine);

  //   let high_light_object = scene.create_content(&mut self.highlight);

  //   pass("high_light_pass")
  //     .with_color( high_light_object_mask.write(), clear(color_same(1.)))
  //     .render_by(high_light_object)
  //     .run(engine, scene);

  //   pass("final_compose")
  //     .with_color(scene_color.write(), clear(color_same(1.)))
  //     .with_color(engine.screen(), clear(color_same(1.)))
  //     .render_by(high_light_blend(high_light_object_mask))
  //     .run(engine, scene);
  // }
}

pub fn pass(name: &'static str) -> PassDescriptor {
  PassDescriptor {
    name,
    phantom: PhantomData,
    channels: Vec::new(),
    tasks: Vec::new(),
    depth_stencil_target: None,
    info: Default::default(),
  }
}

pub struct PassDescriptor<'a> {
  name: &'static str,
  phantom: PhantomData<&'a Attachment<wgpu::TextureFormat>>,
  channels: Vec<(wgpu::Operations<wgpu::Color>, Rc<wgpu::TextureView>)>,
  tasks: Vec<Box<dyn PassContent>>,
  depth_stencil_target: Option<(wgpu::Operations<f32>, Rc<wgpu::TextureView>)>,
  info: PassTargetFormatInfo,
}

#[derive(Clone, Default)]
pub struct PassTargetFormatInfo {
  pub depth_stencil_format: Option<wgpu::TextureFormat>,
  pub color_formats: Vec<wgpu::TextureFormat>,
}

impl<'a> PassDescriptor<'a> {
  #[must_use]
  pub fn with_color(
    mut self,
    attachment: AttachmentWriteView<'a, wgpu::TextureFormat>,
    op: impl Into<wgpu::Operations<wgpu::Color>>,
  ) -> Self {
    self.channels.push((op.into(), attachment.view));
    self.info.color_formats.push(attachment.format);
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
      .replace((op.into(), attachment.view));

    self.info.depth_stencil_format.replace(attachment.format);
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
      .map(|(ops, view)| wgpu::RenderPassColorAttachment {
        view,
        resolve_target: None,
        ops: *ops,
      })
      .collect();

    let depth_stencil_attachment = self.depth_stencil_target.as_ref().map(|(ops, view)| {
      wgpu::RenderPassDepthStencilAttachment {
        view,
        depth_ops: (*ops).into(),
        stencil_ops: None,
      }
    });

    let mut scene = scene.scene.borrow_mut();

    for task in &mut self.tasks {
      task.update(&engine.gpu, &mut scene, &mut resource, &self.info)
    }

    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: self.name.into(),
      color_attachments: color_attachments.as_slice(),
      depth_stencil_attachment,
    });

    for task in &self.tasks {
      task.setup_pass(&mut pass, &scene, &resource, &self.info)
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
