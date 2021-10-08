use std::{cell::RefCell, collections::HashMap, rc::Rc};

use rendiation_algebra::Vec3;
use rendiation_texture::Size;
use rendiation_webgpu::*;

use crate::{RenderPassDispatcher, Scene, StandardForward, ViewerRenderPass, ViewerRenderPassCreator};

pub struct ResourcePoolInner{
  pub attachments: HashMap<(Size, wgpu::TextureFormat), Vec<wgpu::Texture>>,
}


#[derive(Clone)]
pub struct ResourcePool {
  pub inner: Rc<RefCell<ResourcePoolInner>>
}

impl Default for ResourcePool {
  fn default() -> Self {
    todo!()
  }
}

pub struct PassNode {
  render_by: Vec<Box<dyn Renderable>>,
}

pub struct RenderEngine {
  resource: ResourcePool,
  gpu: GPU,
  output_size: Size,
  output: wgpu::TextureView
}

impl RenderEngine {
  pub fn screen(&self) ->  Attachment {
    todo!()
  }
}

pub fn attachment() -> AttachmentDescriptor {
  AttachmentDescriptor {
    format: wgpu::TextureFormat::Rgba8Unorm,
    sizer: default_sizer(),
  }
}

pub fn depth_attachment() -> DepthAttachmentDescriptor {
  DepthAttachmentDescriptor {
    format: wgpu::TextureFormat::Depth24PlusStencil8,
    sizer: default_sizer(),
  }
}

pub struct DepthAttachment {
  pool: ResourcePool,
  des: DepthAttachmentDescriptor,
  id: usize,
}

pub struct DepthAttachmentDescriptor {
  format: wgpu::TextureFormat,
  sizer: Box<dyn FnOnce(Size) -> Size>,
}

fn default_sizer() -> Box<dyn FnOnce(Size) -> Size> {
  Box::new(|size|size)
}

impl DepthAttachmentDescriptor {
  pub fn format(mut self, format: wgpu::TextureFormat) -> Self {
    self.format = format;
    self
  }
}

impl DepthAttachmentDescriptor {
  pub fn request(self, engine: &RenderEngine) -> DepthAttachment {
    let size = (self.sizer)(engine.output_size);
    todo!()
    
  }
}

pub struct Attachment {
  pool: ResourcePool,
  des: AttachmentDescriptor,
  id: usize,
}

pub struct AttachmentDescriptor {
  format: wgpu::TextureFormat,
  sizer: Box<dyn FnOnce(Size) -> Size>,
}

impl AttachmentDescriptor {
  pub fn format(mut self, format: wgpu::TextureFormat) -> Self {
    self.format = format;
    self
  }
}

impl AttachmentDescriptor {
  pub fn request(self, engine: &RenderEngine) -> Attachment {
    let size = (self.sizer)(engine.output_size);
    todo!()
  }
}

pub trait Pipeline {
  fn render(&mut self, engine: &RenderEngine, scene: &mut Scene);
}

pub struct HighLight{
  color: Vec3<f32>
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


impl ViewerRenderPassCreator for HighLight {
  type TargetResource = wgpu::TextureView;

  fn create_pass<'a>(
    &'a self,
    scene: &Scene,
    target: &'a Self::TargetResource,
    encoder: &'a mut wgpu::CommandEncoder,
  ) -> wgpu::RenderPass<'a> {
    todo!()
  }
}


pub struct SimplePipeline {
  forward: StandardForward, 
  highlight: HighLight,
}

impl Scene {
  pub fn create_pass<P>(&mut self, pass: &mut P) -> RenderPassDispatcher<P> {
    // RenderPassDispatcher {
    //     scene: self,
    //     pass,
    //   }
    todo!()
  }
}

impl Pipeline for SimplePipeline {
  #[rustfmt::skip]
  fn render(&mut self, engine: &RenderEngine, scene: &mut Scene) {
    let scene_main_content = scene.create_pass(&mut self.forward);  

    let mut scene_color = attachment()
    .format(wgpu::TextureFormat::Rgba8Unorm)
    .request(engine);

    let mut scene_depth = depth_attachment()
    .format(wgpu::TextureFormat::Depth32Float)
    .request(engine);

    pass("scene_pass")
      .with_color(&mut scene_color, clear(color(0.1, 0.2, 0.3)))
      .with_depth(&mut scene_depth, clear(1.))
      .render_by(scene_main_content)
      .run(engine);

    let mut high_light_object_mask = attachment()
    .format(wgpu::TextureFormat::Rgba8Unorm)
    .request(engine);

     
    let high_light_object = scene.create_pass(&mut self.highlight); 

    pass("high_light_pass")
      .with_color(&mut high_light_object_mask, clear(color_same(1.)))
      .render_by(high_light_object)
      .run(engine);


    pass("final_compose")
      // .with_color(&mut scene_color, clear(color(0.1, 0.2, 0.3)))
      .with_color(&mut engine.screen(), clear(color_same(1.)))
      .render_by(copy(&mut scene_color))
      .render_by(high_light_blend(&mut high_light_object_mask))
      .run(engine);
  }
}


pub struct HiLighter<'a> {
  source: &'a mut Attachment,
}

impl<'a> Renderable for HiLighter<'a> {
  fn setup_pass<'r>(&'r self, pass: &mut wgpu::RenderPass<'r>) {
    todo!()
  }
}

pub fn high_light_blend<'a>(source: &'a mut Attachment) -> HiLighter<'a> {
  todo!()
}

pub struct Copier<'a> {
  source: &'a mut Attachment,
}

impl<'a> Renderable for Copier<'a> {
  fn setup_pass<'r>(&'r self, pass: &mut wgpu::RenderPass<'r>) {
    todo!()
  }
}

pub fn copy<'a>(source: &'a mut Attachment) -> Copier<'a> {
  todo!()
}

pub fn pass(name: &'static str) -> PassDescriptor {
  PassDescriptor {
    name,
    channels: Vec::new(),
  }
}

pub struct PassDescriptor {
  name: &'static str,
  channels: Vec<(wgpu::Operations<f32>, usize)>,
}

impl ViewerRenderPass for PassDescriptor {
  fn depth_stencil_format(&self) -> Option<wgpu::TextureFormat> {
   todo!()
  }

  fn color_format(&self) -> &[wgpu::TextureFormat] {
    // self.color_format.as_slice()
    todo!()
  }
}


impl ViewerRenderPassCreator for PassDescriptor {
  type TargetResource = wgpu::TextureView;

  fn create_pass<'a>(
    &'a self,
    scene: &Scene,
    target: &'a Self::TargetResource,
    encoder: &'a mut wgpu::CommandEncoder,
  ) -> wgpu::RenderPass<'a> {
    todo!()
  }
}

impl PassDescriptor {
  #[must_use]
  pub fn with_color(
    self,
    attachment: &mut Attachment,
    op: impl Into<wgpu::Operations<wgpu::Color>>,
  ) -> Self {
    self
  }

  #[must_use]
  pub fn with_depth(
    self,
    attachment: &mut DepthAttachment,
    op: impl Into<wgpu::Operations<f32>>,
  ) -> Self {
    self
  }

  #[must_use]
  pub fn render_by(self, renderable: impl Renderable) -> Self {
    self
  }

  pub fn run(self, engine: &RenderEngine) {
    // engine.gpu.render(RenderPassDispatcher{
    //     scene: todo!(),
    //     pass: todo!(),
    // })
  }
}

pub fn color(r: f32, g: f32, b: f32) -> wgpu::Color {
  todo!()
}

pub fn color_same(r: f32) -> wgpu::Color {
  // or use marco?
  todo!()
}

pub fn clear<V>(v: V) -> Operations<V> {
  wgpu::Operations {
    load: wgpu::LoadOp::Clear(v),
    store: true,
  }
}
