use super::*;

pub struct StandardForward {
  depth: wgpu::Texture,
  depth_view: wgpu::TextureView,
  color_format: [wgpu::TextureFormat; 1],
}

impl ViewerRenderPass for StandardForward {
  fn depth_stencil_format(&self) -> Option<wgpu::TextureFormat> {
    wgpu::TextureFormat::Depth32Float.into()
  }

  fn color_format(&self) -> &[wgpu::TextureFormat] {
    self.color_format.as_slice()
  }
}

impl StandardForward {
  pub fn depth_format() -> wgpu::TextureFormat {
    wgpu::TextureFormat::Depth32Float
  }
}

impl Scene {
  fn get_main_pass_load_op(&self) -> wgpu::LoadOp<wgpu::Color> {
    if let Some(clear_color) = self.background.require_pass_clear() {
      return wgpu::LoadOp::Clear(clear_color);
    }

    return wgpu::LoadOp::Load;
  }
}

impl StandardForward {
  pub fn new(gpu: &GPU, target_format: wgpu::TextureFormat, size: (f32, f32)) -> Self {
    let (depth, depth_view) = Self::create_gpu(gpu, size);

    Self {
      depth,
      depth_view,
      color_format: [target_format],
    }
  }
  fn create_gpu(gpu: &GPU, size: (f32, f32)) -> (wgpu::Texture, wgpu::TextureView) {
    let depth = gpu.device.create_texture(&wgpu::TextureDescriptor {
      size: wgpu::Extent3d {
        width: size.0 as u32,
        height: size.1 as u32,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: Self::depth_format(),
      usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
      label: None,
    });

    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
    (depth, depth_view)
  }

  pub fn resize(&mut self, gpu: &GPU, size: (f32, f32)) {
    let (depth, depth_view) = Self::create_gpu(gpu, size);
    self.depth = depth;
    self.depth_view = depth_view;
  }
}

impl ViewerRenderPassCreator for StandardForward {
  type TargetResource = wgpu::SwapChainFrame;

  fn create_pass<'a>(
    &'a self,
    scene: &Scene,
    target: &'a Self::TargetResource,
    encoder: &'a mut wgpu::CommandEncoder,
  ) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: "scene pass".into(),
      color_attachments: &[wgpu::RenderPassColorAttachment {
        view: &target.output.view,
        resolve_target: None,
        ops: wgpu::Operations {
          load: scene.get_main_pass_load_op(),
          store: true,
        },
      }],
      depth_stencil_attachment: wgpu::RenderPassDepthStencilAttachment {
        view: &self.depth_view,
        depth_ops: wgpu::Operations {
          load: wgpu::LoadOp::Clear(1.),
          store: true,
        }
        .into(),
        stencil_ops: None,
      }
      .into(),
    })
  }
}
