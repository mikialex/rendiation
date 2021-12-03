use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc};

use interphaser::FrameTarget;
use rendiation_texture::Size;
use rendiation_webgpu::*;

use crate::*;

#[derive(Default)]
pub struct ResourcePoolImpl {
  pub attachments: HashMap<(Size, wgpu::TextureFormat, u32), Vec<wgpu::Texture>>,
}

#[derive(Clone, Default)]
pub struct ResourcePool {
  pub inner: Rc<RefCell<ResourcePoolImpl>>,
}

pub struct RenderEngine {
  resource: ResourcePool,
  gpu: Rc<GPU>,
  msaa_sample_count: u32,
  pub output: Option<FrameTarget>,
}

impl RenderEngine {
  pub fn new(gpu: Rc<GPU>) -> Self {
    Self {
      resource: Default::default(),
      output: Default::default(),
      msaa_sample_count: 4,
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
      sample_count: 1,
    }
  }

  pub fn multisampled_attachment(&self) -> AttachmentDescriptor<wgpu::TextureFormat> {
    AttachmentDescriptor {
      format: wgpu::TextureFormat::Rgba8Unorm,
      sample_count: self.msaa_sample_count,
      sizer: default_sizer(),
    }
  }

  pub fn multisampled_depth_attachment(&self) -> AttachmentDescriptor<wgpu::TextureFormat> {
    AttachmentDescriptor {
      format: wgpu::TextureFormat::Depth24PlusStencil8,
      sample_count: self.msaa_sample_count,
      sizer: default_sizer(),
    }
  }
}

pub fn attachment() -> AttachmentDescriptor<wgpu::TextureFormat> {
  AttachmentDescriptor {
    format: wgpu::TextureFormat::Rgba8Unorm,
    sample_count: 1,
    sizer: default_sizer(),
  }
}

pub fn depth_attachment() -> AttachmentDescriptor<wgpu::TextureFormat> {
  AttachmentDescriptor {
    format: wgpu::TextureFormat::Depth24PlusStencil8,
    sample_count: 1,
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
  once: bool,
}

pub type ColorAttachment = Attachment<wgpu::TextureFormat>;
pub type DepthAttachment = Attachment<wgpu::TextureFormat>; // todo

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
      sample_count: self.des.sample_count,
    }
  }

  pub fn read(&self) -> AttachmentReadView<F> {
    assert_eq!(self.des.sample_count, 1); // todo support latter
    AttachmentReadView {
      phantom: PhantomData,
      view: Rc::new(
        self
          .texture
          .as_ref()
          .unwrap()
          .create_view(&wgpu::TextureViewDescriptor::default()),
      ),
    }
  }

  pub fn read_into(self) -> AttachmentOwnedReadView<F> {
    assert_eq!(self.des.sample_count, 1); // todo support latter
    let view = self
      .texture
      .as_ref()
      .unwrap()
      .create_view(&wgpu::TextureViewDescriptor::default());
    AttachmentOwnedReadView {
      _att: self,
      view: Rc::new(view),
    }
  }
}

impl<F: AttachmentFormat> Drop for Attachment<F> {
  fn drop(&mut self) {
    if let Ok(texture) = Rc::try_unwrap(self.texture.take().unwrap()) {
      let mut pool = self.pool.inner.borrow_mut();
      let cached = pool
        .attachments
        .entry((self.size, self.des.format.into(), self.des.sample_count))
        .or_insert_with(Default::default);

      if !self.once {
        cached.push(texture)
      }
    }
  }
}

pub struct AttachmentWriteView<'a, F: AttachmentFormat> {
  phantom: PhantomData<&'a Attachment<F>>,
  size: Size,
  view: Rc<wgpu::TextureView>, // todo opt enum
  format: F,
  sample_count: u32,
}

pub struct AttachmentReadView<'a, F: AttachmentFormat> {
  phantom: PhantomData<&'a Attachment<F>>,
  view: Rc<wgpu::TextureView>,
}

impl<'a, F: AttachmentFormat> BindableResource for AttachmentReadView<'a, F> {
  fn as_bindable(&self) -> wgpu::BindingResource {
    wgpu::BindingResource::TextureView(self.view.as_ref())
  }

  fn bind_layout() -> wgpu::BindingType {
    wgpu::BindingType::Texture {
      multisampled: false,
      sample_type: wgpu::TextureSampleType::Float { filterable: true },
      view_dimension: wgpu::TextureViewDimension::D2,
    }
  }
}

pub struct AttachmentOwnedReadView<F: AttachmentFormat> {
  _att: Attachment<F>,
  view: Rc<wgpu::TextureView>,
}

impl<F: AttachmentFormat> BindableResource for AttachmentOwnedReadView<F> {
  fn as_bindable(&self) -> wgpu::BindingResource {
    wgpu::BindingResource::TextureView(self.view.as_ref())
  }

  fn bind_layout() -> wgpu::BindingType {
    wgpu::BindingType::Texture {
      multisampled: false,
      sample_type: wgpu::TextureSampleType::Float { filterable: true },
      view_dimension: wgpu::TextureViewDimension::D2,
    }
  }
}

#[derive(Clone)]
pub struct AttachmentDescriptor<F> {
  format: F,
  sample_count: u32,
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
      .entry((size, self.format.into(), self.sample_count))
      .or_insert_with(Default::default);
    let texture = cached.pop().unwrap_or_else(|| {
      engine.gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: size.into_gpu_size(),
        mip_level_count: 1,
        sample_count: self.sample_count,
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
      once: false,
    }
  }

  /// Some impl issue on metal for reusing msaa resolve texture
  pub fn request_once(self, engine: &RenderEngine) -> Attachment<F> {
    let size = (self.sizer)(engine.output.as_ref().unwrap().size);
    let texture = engine.gpu.device.create_texture(&wgpu::TextureDescriptor {
      label: None,
      size: size.into_gpu_size(),
      mip_level_count: 1,
      sample_count: self.sample_count,
      dimension: TextureDimension::D2,
      format: self.format.into(),
      usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
    });

    Attachment {
      pool: engine.resource.clone(),
      des: self,
      size,
      texture: Rc::new(texture).into(),
      once: true,
    }
  }
}

pub trait PassContent {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, pass_info: &RenderPassInfo);
  fn setup_pass<'a>(&'a self, pass: &mut GPURenderPass<'a>, scene: &'a Scene);
}

impl<T: PassContent> PassContent for Option<T> {
  fn update(&mut self, gpu: &GPU, scene: &mut Scene, pass_info: &RenderPassInfo) {
    if let Some(c) = self {
      c.update(gpu, scene, pass_info);
    }
  }

  fn setup_pass<'a>(&'a self, pass: &mut GPURenderPass<'a>, scene: &'a Scene) {
    if let Some(c) = self {
      c.setup_pass(pass, scene);
    }
  }
}

pub struct SimplePipeline {
  forward: ForwardScene,
  highlight: HighLighter,
  background: BackGroundRendering,
}

impl SimplePipeline {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      forward: Default::default(),
      highlight: HighLighter::new(gpu),
      background: Default::default(),
    }
  }
}

impl SimplePipeline {
  #[rustfmt::skip]
  #[allow(clippy::logic_bug)]
  pub fn render_simple(&mut self, engine: &RenderEngine, content: &mut Viewer3dContent) {
    let scene = &mut content.scene;

    let mut scene_depth = depth_attachment().request(engine);

    let mut msaa_color = engine.multisampled_attachment().request(engine);
    let mut msaa_depth = engine.multisampled_depth_attachment().request(engine);

    let mut widgets_result = attachment().request(engine);

    pass("scene-widgets")
      .with_color(msaa_color.write(), clear(all_zero()))
      .with_depth(msaa_depth.write(), clear(1.))
      .resolve_to(widgets_result.write())
      .render_by(&mut content.axis)
      .run(engine, scene);

    let mut final_compose = pass("compose-all")
      .with_color(engine.screen(), scene.get_main_pass_load_op())
      .with_depth(scene_depth.write(), clear(1.));

    final_compose
      .render(&mut self.background)
      .render(&mut self.forward);

    let mut highlight_compose = (!content.selections.is_empty()).then(||{
       let mut selected = attachment()
        .format(wgpu::TextureFormat::Rgba8Unorm)
        .request(engine);

      pass("highlight-selected-mask")
        .with_color(selected.write(), clear(color_same(0.)))
        .render_by(&mut highlight(&content.selections))
        .run(engine, scene);

      self.highlight.draw(selected.read_into())
    });

    let mut copy_frame = copy_frame(widgets_result.read_into());

    final_compose
      .render(&mut highlight_compose)
      .render(&mut copy_frame);

    final_compose.run(engine, scene);

  }
}

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

    for task in &mut self.tasks {
      task.update(&engine.gpu, scene, &info)
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
