use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc};

use interphaser::FrameTarget;
use rendiation_texture::Size;
use rendiation_webgpu::*;

use crate::*;

#[derive(Default)]
pub struct ResourcePoolImpl {
  pub attachments: HashMap<(Size, wgpu::TextureFormat), Vec<wgpu::Texture>>,
}

#[derive(Clone, Default)]
pub struct ResourcePool {
  pub inner: Rc<RefCell<ResourcePoolImpl>>,
}

pub struct RenderEngine {
  resource: ResourcePool,
  gpu: Rc<GPU>,
  pub output: Option<FrameTarget>,
}

impl RenderEngine {
  pub fn new(gpu: Rc<GPU>) -> Self {
    Self {
      resource: Default::default(),
      output: Default::default(),
      gpu,
    }
  }

  pub fn notify_output_resized(&self) {
    self.resource.inner.borrow_mut().attachments.clear();
  }

  pub fn screen(&self) -> AttachmentWriteView<wgpu::TextureFormat> {
    let output = self.output.as_ref().unwrap();
    AttachmentWriteView {
      phantom: PhantomData,
      size: output.size,
      view: output.view.clone(),
      format: output.format,
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
      size: self.size,
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
  size: Size,
  view: Rc<wgpu::TextureView>, // todo opt enum
  format: F,
}

// pub struct AttachmentReadView<'a, F: AttachmentFormat> {
//   attachment: &'a Attachment<F>,
//   view: wgpu::TextureView,
// }

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
    let size = (self.sizer)(engine.output.as_ref().unwrap().size);
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

pub trait PassContent {
  fn update(
    &mut self,
    gpu: &GPU,
    scene: &mut Scene,
    resource: &mut ResourcePoolImpl,
    pass_info: &RenderPassInfo,
  );
  fn setup_pass<'a>(&'a self, pass: &mut GPURenderPass<'a>, scene: &'a Scene);
}

#[derive(Default)]
pub struct SimplePipeline {
  forward: ForwardScene,
  // highlight: HighLight,
}

impl SimplePipeline {
  #[rustfmt::skip]
  pub fn render_simple(&mut self, engine: &RenderEngine, content: &mut Viewer3dContent) {
    let scene = &mut content.scene;

    let mut scene_depth = depth_attachment()
      .format(wgpu::TextureFormat::Depth24PlusStencil8)
      .request(engine);

    pass("forward-group")
      .with_color(engine.screen(), scene.get_main_pass_load_op())
      .with_depth(scene_depth.write(), clear(1.))
      .render_by(&mut BackGroundRendering)
      .render_by(&mut self.forward)
      .render_by(&mut content.axis)
      .run(engine, scene);

  }

  // #[rustfmt::skip]
  // pub fn render(&mut self, engine: &RenderEngine, scene: &mut Scene) {

  //   let mut scene_color = attachment()
  //     .format(wgpu::TextureFormat::Rgba8Unorm)
  //     .request(engine);

  //   let mut scene_depth = depth_attachment()
  //     .format(wgpu::TextureFormat::Depth32Float)
  //     .request(engine);

  //   pass("scene_pass")
  //     .with_color(scene_color.write(), scene.get_main_pass_load_op())
  //     .with_depth(scene_depth.write(), clear(1.))
  //     .render_by(&mut BackGroundRendering)
  //     .render_by(&mut self.forward)
  //     .run(engine, scene);

  //   let mut high_light_object_mask = attachment()
  //     .format(wgpu::TextureFormat::Rgba8Unorm)
  //     .request(engine);

  //   // let high_light_object = scene.create_content(&mut self.highlight);
  //   let high_light_object = &mut BackGroundRendering;

  //   pass("high_light_pass")
  //     .with_color( high_light_object_mask.write(), clear(color_same(1.)))
  //     .render_by(high_light_object)
  //     .run(engine, scene);

  //   pass("final_compose")
  //     .with_color(scene_color.write(), clear(color_same(1.)))
  //     .with_color(engine.screen(), clear(color_same(1.)))
  //     .render_by(&mut high_light_blend(high_light_object_mask))
  //     .run(engine, scene);
  // }
}

#[allow(clippy::field_reassign_with_default)]
pub fn pass<'t>(name: impl Into<String>) -> PassDescriptor<'static, 't> {
  let mut desc = RenderPassDescriptorOwned::default();
  desc.name = name.into();
  PassDescriptor {
    phantom: PhantomData,
    tasks: Vec::new(),
    desc,
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
    self
  }

  #[must_use]
  pub fn render_by(mut self, renderable: &'t mut dyn PassContent) -> Self {
    self.tasks.push(renderable);
    self
  }

  pub fn run(mut self, engine: &RenderEngine, scene: &mut Scene) {
    let mut resource = engine.resource.inner.borrow_mut();

    let mut encoder = engine.gpu.encoder.borrow_mut();

    let info = RenderPassInfo {
      buffer_size: self.desc.channels.first().unwrap().2,
      format_info: self.desc.info.clone(),
    };

    for task in &mut self.tasks {
      task.update(&engine.gpu, scene, &mut resource, &info)
    }

    let mut pass = encoder.begin_render_pass(&self.desc);

    let camera = scene.active_camera.as_ref().unwrap();
    camera.bounds.setup_viewport(&mut pass);

    for task in &self.tasks {
      task.setup_pass(&mut pass, scene)
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
