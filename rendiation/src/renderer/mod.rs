pub mod attachment_texture;
pub mod bindgroup;
pub mod buffer;
pub mod consts;
pub mod pipeline;
pub mod render_pass;
pub mod sampler;
pub mod shader_util;
pub mod texture;

pub use attachment_texture::*;
pub use bindgroup::*;
pub use buffer::*;
pub use pipeline::*;
pub use render_pass::*;
pub use sampler::*;
pub use texture::*;

/// The renderer trait.
///
/// Impl this trait for build your own renderer.
pub trait Renderer: 'static + Sized {
  fn init(device: &wgpu::Device, size: (usize, usize)) -> Self;
  fn resize(&mut self, device: &wgpu::Device, size: (usize, usize));
  fn render();
}

/// WebGPU renderer backend
///
/// the backend render not contains any specific render resource.
/// just encapsulate webgpu functionality
pub struct WGPURenderer {
  surface: wgpu::Surface,
  pub adapter: wgpu::Adapter,
  pub device: wgpu::Device,
  pub queue: Queue,
  pub encoder: wgpu::CommandEncoder,
  pub size: (usize, usize),
  pub hidpi_factor: f32,
  pub swap_chain: SwapChain,
}

pub struct Queue(pub wgpu::Queue);
impl Queue {
  pub fn submit(&mut self, device: &wgpu::Device, old_encoder: &mut wgpu::CommandEncoder) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
    use std::mem;
    mem::swap(&mut encoder, old_encoder);

    let command_buf = encoder.finish();
    self.0.submit(&[command_buf]);
  }
}

pub struct SwapChain {
  pub swap_chain: wgpu::SwapChain,
  pub swap_chain_descriptor: wgpu::SwapChainDescriptor,
}

impl SwapChain {
  pub fn new(surface: &wgpu::Surface, size: (usize, usize), device: &wgpu::Device) -> Self {
    let swap_chain_descriptor = wgpu::SwapChainDescriptor {
      usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
      format: wgpu::TextureFormat::Bgra8UnormSrgb,
      width: size.0 as u32,
      height: size.1 as u32,
      present_mode: wgpu::PresentMode::Vsync,
    };
    let swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);
    Self {
      swap_chain_descriptor,
      swap_chain,
    }
  }

  pub fn request_output(&mut self) -> wgpu::SwapChainOutput {
    self.swap_chain.get_next_texture()
  }
}

impl WGPURenderer {
  pub fn new(surface: wgpu::Surface, size: (usize, usize), hidpi_factor: f32) -> Self {
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
    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
    let swap_chain = SwapChain::new(&surface, size, &device);
    Self {
      surface,
      adapter,
      device,
      queue: Queue(queue),
      encoder,
      size,
      hidpi_factor,
      swap_chain,
    }
  }

  pub fn resize(&mut self, size: (usize, usize)) {
    self.swap_chain.swap_chain_descriptor.width = size.0 as u32;
    self.swap_chain.swap_chain_descriptor.height = size.1 as u32;
    self.swap_chain.swap_chain = self
      .device
      .create_swap_chain(&self.surface, &self.swap_chain.swap_chain_descriptor);
    self.size = size;
  }

  pub fn create_buffer<D: 'static + Copy>(
    &self,
    data: &[D],
    usage: wgpu::BufferUsage,
  ) -> WGPUBuffer {
    WGPUBuffer::new(&self.device, &data, usage)
  }
  pub fn create_index_buffer<D: 'static + Copy>(&self, data: &[D]) -> WGPUBuffer {
    WGPUBuffer::new(&self.device, &data, wgpu::BufferUsage::INDEX)
  }
  pub fn create_vertex_buffer<D: 'static + Copy>(&self, data: &[D]) -> WGPUBuffer {
    WGPUBuffer::new(&self.device, &data, wgpu::BufferUsage::VERTEX)
  }

}
