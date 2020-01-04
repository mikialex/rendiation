use std::collections::HashMap;

pub mod attachment_texture;
pub mod bindgroup;
pub mod buffer;
pub mod r#const;
pub mod pipeline;
pub mod sampler;
pub mod shader_util;
pub mod texture;

pub use attachment_texture::*;
pub use bindgroup::*;
pub use buffer::*;
pub use pipeline::*;
pub use sampler::*;
pub use texture::*;

pub struct WGPURenderer {
  surface: wgpu::Surface,
  adapter: wgpu::Adapter,
  device: wgpu::Device,
  queue: wgpu::Queue,
  // pipelines: HashMap<String, WGPUPipeline>,
  // depth: WGPUAttachmentTexture,

  swap_chain: wgpu::SwapChain,
  swap_chain_descriptor: wgpu::SwapChainDescriptor,
}

impl WGPURenderer {
  pub fn new(surface: wgpu::Surface, size: (usize, usize)) -> Self {
    let adapter = wgpu::Adapter::request(&wgpu::RequestAdapterOptions {
      power_preference: wgpu::PowerPreference::Default,
      backends: wgpu::BackendBit::PRIMARY,
    })
    .unwrap();

    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
      extensions: wgpu::Extensions {
        anisotropic_filtering: false,
      },
      limits: wgpu::Limits::default(),
    });
    let swap_chain_descriptor = wgpu::SwapChainDescriptor {
      usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
      format: wgpu::TextureFormat::Bgra8UnormSrgb,
      width: size.0 as u32,
      height: size.1 as u32,
      present_mode: wgpu::PresentMode::Vsync,
    };
    let swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);

    Self{
      surface,
      adapter,
      device,
      queue,
      // pipelines: HashMap<String, WGPUPipeline>,
      // depth: WGPUAttachmentTexture,

      swap_chain,
      swap_chain_descriptor,
    }
  }

  pub fn resize(&mut self, width: usize, height: usize) {
    // self.depth.resize(&self.device, width, height)
  }

  pub fn render(&mut self) {
    let frame = self.swap_chain.get_next_texture();
    let mut encoder = self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
    // current_frame: wgpu::SwapChainOutput,

    encoder.finish();
  }
}
