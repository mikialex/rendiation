use super::*;

pub struct StandardForward {
  depth: wgpu::Texture,
  depth_view: wgpu::TextureView,
  color_format: [wgpu::TextureFormat; 1],
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
  pub fn new(gpu: &GPU, target_format: wgpu::TextureFormat, size: (u32, u32)) -> Self {
    let (depth, depth_view) = Self::create_gpu(gpu, size);

    Self {
      depth,
      depth_view,
      color_format: [target_format],
    }
  }
  fn create_gpu(gpu: &GPU, size: (u32, u32)) -> (wgpu::Texture, wgpu::TextureView) {
    let depth = gpu.device.create_texture(&wgpu::TextureDescriptor {
      size: wgpu::Extent3d {
        width: size.0,
        height: size.1,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: Self::depth_format(),
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      label: None,
    });

    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
    (depth, depth_view)
  }

  pub fn resize(&mut self, gpu: &GPU, size: (u32, u32)) {
    let (depth, depth_view) = Self::create_gpu(gpu, size);
    self.depth = depth;
    self.depth_view = depth_view;
  }
}
