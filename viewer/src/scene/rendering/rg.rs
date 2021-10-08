use std::collections::HashMap;

use rendiation_texture::Size;
use rendiation_webgpu::*;

use crate::Scene;

pub struct ResourcePool {
  pub textures: HashMap<String, Texture>,
  pub buffers: HashMap<String, Buffer>,
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
}

impl RenderEngine {
  //
}

pub fn attachment() -> AttachmentDescriptor {
  //
  todo!()
}

pub fn depth_attachment() -> DepthAttachmentDescriptor {
  //
  todo!()
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

impl DepthAttachmentDescriptor {
  pub fn format(self, format: wgpu::TextureFormat) -> Self {
    self
  }
}

impl DepthAttachmentDescriptor {
  pub fn request(self, engine: &RenderEngine) -> DepthAttachment {
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
  pub fn format(self, format: wgpu::TextureFormat) -> Self {
    self
  }
}

impl AttachmentDescriptor {
  pub fn request(self, engine: &RenderEngine) -> Attachment {
    todo!()
  }
}

#[rustfmt::skip]
fn pipeline(engine: &RenderEngine, scene: &Scene) {

  let scene_main_content = todo!();  
  let high_light_object = todo!();

  let scene_color = attachment()
  .format(wgpu::TextureFormat::Rgba8Unorm)
  .request(engine);

  let scene_depth = depth_attachment()
  .format(wgpu::TextureFormat::Depth32Float)
  .request(engine);

  pass("scene_pass")
    .with_color(&mut scene_color, clear(color(0.1, 0.2, 0.3)))
    .with_depth(&mut scene_depth, clear(1.))
    .render_by(&scene_main_content);

  let high_light_object_mask = attachment()
  .format(wgpu::TextureFormat::Rgba8Unorm)
  .request(engine);

  pass("high_light_pass")
    .with_color(&mut high_light_object_mask, clear(color_same(1.)))
    .render_by(&high_light_object);


  pass("final_compose")
    // .with_color(&mut scene_color, clear(color(0.1, 0.2, 0.3)))
    .with_color(engine.screen(), clear(color_same(1.)))
    .render_by(copy(&mut scene_color))
    .render_by(high_light_blend(&mut high_light_object_mask))
    .run(engine);

}

pub struct HiLighter {
  //
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

impl PassDescriptor {
  pub fn with_color(
    self,
    attachment: &mut Attachment,
    op: impl Into<wgpu::Operations<wgpu::Color>>,
  ) -> Self {
    self
  }

  pub fn with_depth(
    self,
    attachment: &mut DepthAttachment,
    op: impl Into<wgpu::Operations<f32>>,
  ) -> Self {
    self
  }

  pub fn render_by(self, renderable: impl Renderable) -> Self {
    self
  }

  pub fn run(self, engine: &RenderEngine) {
    //
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
