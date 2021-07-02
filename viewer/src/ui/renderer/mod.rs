use rendiation_algebra::Vec2;

use crate::renderer::{RenderPassCreator, Renderable, Renderer};

#[derive(Debug, Clone)]
pub enum Primitive {
  Quad(Quad),
  Text(TextInfo),
}

#[derive(Debug, Clone)]
pub struct Quad {
  x: f32,
  y: f32,
  width: f32,
  height: f32,
}

#[derive(Debug, Clone)]
pub struct TextInfo {
  content: String,
  max_width: Option<f32>,
  x: f32,
  y: f32,
}

impl Primitive {
  pub fn test_pointer_in(&self, pointer: Vec2<f32>) -> bool {
    match self {
      Primitive::Quad(_) => todo!(),
      Primitive::Text(_) => todo!(),
    }
  }
}

pub struct WebGPUxUIRenderPass<'a> {
  texture_cache: &'a mut UITextureCache,
}

pub struct UITextureCache {
  cached_frame: wgpu::TextureView,
  cached: wgpu::Texture,
}

impl<'r> RenderPassCreator<wgpu::TextureView> for WebGPUxUIRenderPass<'r> {
  fn create<'a>(
    &'a self,
    view: &'a wgpu::TextureView,
    encoder: &'a mut wgpu::CommandEncoder,
  ) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: "ui pass".into(),
      color_attachments: &[wgpu::RenderPassColorAttachment {
        view,
        resolve_target: None,
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
          store: true,
        },
      }],
      depth_stencil_attachment: None,
    })
  }
}

impl<'r> Renderable for WebGPUxUIRenderPass<'r> {
  fn update(&mut self, renderer: &mut Renderer, encoder: &mut wgpu::CommandEncoder) {
    todo!()
  }

  fn setup_pass<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
    todo!()
  }
}
