use rendiation::*;

pub struct TestRenderer{
  pub depth: WGPUAttachmentTexture,
}

impl Renderer for TestRenderer{
  
  fn init(device: &wgpu::Device, size: (usize, usize)) -> Self {
    let depth = WGPUAttachmentTexture::new_as_depth(&device, wgpu::TextureFormat::Depth32Float, size);
    Self{
      depth
    }
  }
  fn resize(&mut self, device: &wgpu::Device, size: (usize, usize)){
    self.depth.resize(device, size.0, size.1)
  }
}