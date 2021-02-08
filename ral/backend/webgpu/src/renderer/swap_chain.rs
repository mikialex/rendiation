use crate::WGPURenderer;

pub struct SwapChain {
  surface: wgpu::Surface,
  pub swap_chain: wgpu::SwapChain,
  pub swap_chain_descriptor: wgpu::SwapChainDescriptor,
  pub size: (usize, usize),
}

impl SwapChain {
  pub fn new(surface: wgpu::Surface, size: (usize, usize), renderer: &WGPURenderer) -> Self {
    let swap_chain_descriptor = wgpu::SwapChainDescriptor {
      usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
      format: renderer.swap_chain_format,
      width: size.0 as u32,
      height: size.1 as u32,
      present_mode: wgpu::PresentMode::Fifo,
    };
    let swap_chain = renderer
      .device
      .create_swap_chain(&surface, &swap_chain_descriptor);
    Self {
      surface,
      swap_chain_descriptor,
      swap_chain,
      size,
    }
  }

  pub fn resize(&mut self, mut size: (usize, usize), device: &wgpu::Device) {
    if size.0 == 0 {
      size.0 = 1;
    }

    if size.1 == 0 {
      size.1 = 1;
    }

    self.swap_chain_descriptor.width = size.0 as u32;
    self.swap_chain_descriptor.height = size.1 as u32;
    self.swap_chain = device.create_swap_chain(&self.surface, &self.swap_chain_descriptor);
    self.size = size;
  }

  pub fn get_current_frame(&mut self) -> wgpu::SwapChainTexture {
    self.swap_chain.get_current_frame().unwrap().output
  }
}
