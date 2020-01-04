pub struct WGPUAttachmentTexture {
  descriptor: wgpu::TextureDescriptor,
  gpu_texture: wgpu::Texture,
  view: wgpu::TextureView,
}

impl WGPUAttachmentTexture {
  pub fn new_as_depth(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: (usize, usize),
  ) -> Self {
    let descriptor = wgpu::TextureDescriptor {
      size: wgpu::Extent3d {
        width: size.0 as u32,
        height: size.1 as u32,
        depth: 1,
      },
      array_layer_count: 1,
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format,
      usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
    };
    let depth_texture = device.create_texture(&descriptor);
    let view = depth_texture.create_default_view();
    Self {
      descriptor,
      gpu_texture: depth_texture,
      view,
    }
  }

  pub fn resize(&mut self, device: &wgpu::Device, width: usize, height: usize) {
    self.descriptor.size.width = width as u32;
    self.descriptor.size.height = height as u32;
    self.gpu_texture = device.create_texture(&self.descriptor);
    self.view = self.gpu_texture.create_default_view();
  }
}
