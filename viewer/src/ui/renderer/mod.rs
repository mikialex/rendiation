use crate::renderer::{RenderPassCreator, Renderable, Renderer};

pub struct WebGPUxUIRenderPass<'a> {
  texture_cache: &'a mut UITextureCache,
}

pub struct UITextureCache {
  cached_target_frame: wgpu::TextureView,
  cached_target: wgpu::Texture,
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

pub struct GPUxUISolidQuad {
  uniform: wgpu::Buffer,
  bindgroup: wgpu::BindGroup,
}

pub enum GPUxUIPrimitive {
  Quad(GPUxUISolidQuad),
}

pub struct WebGPUxUIRenderer {
  texture_cache: UITextureCache,
  gpu_primitive_cache: Vec<GPUxUIPrimitive>,
  solid_quad_pipeline: wgpu::RenderPipeline,
}

impl WebGPUxUIRenderer {
  pub fn new(device: &wgpu::Device) -> Self {
    todo!()
  }
}
