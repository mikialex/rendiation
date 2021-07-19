use rendiation_texture::Size;

pub struct SwapChain {
  surface: wgpu::Surface,
  pub swap_chain: wgpu::SwapChain,
  pub swap_chain_descriptor: wgpu::SwapChainDescriptor,
  pub size: Size,
}

impl SwapChain {
  pub fn new(
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    surface: wgpu::Surface,
    size: Size,
  ) -> Self {
    let swap_chain_descriptor = wgpu::SwapChainDescriptor {
      usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
      format: adapter.get_swap_chain_preferred_format(&surface).unwrap(),
      width: Into::<usize>::into(size.width) as u32,
      height: Into::<usize>::into(size.height) as u32,
      present_mode: wgpu::PresentMode::Fifo,
    };
    let swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);
    Self {
      surface,
      swap_chain_descriptor,
      swap_chain,
      size,
    }
  }

  pub fn resize(&mut self, mut size: Size, device: &wgpu::Device) {
    self.swap_chain_descriptor.width = Into::<usize>::into(size.width) as u32;
    self.swap_chain_descriptor.height = Into::<usize>::into(size.height) as u32;
    self.swap_chain = device.create_swap_chain(&self.surface, &self.swap_chain_descriptor);
    self.size = size;
  }

  pub fn get_current_frame(&mut self) -> Result<wgpu::SwapChainFrame, wgpu::SwapChainError> {
    self.swap_chain.get_current_frame()
  }
}
