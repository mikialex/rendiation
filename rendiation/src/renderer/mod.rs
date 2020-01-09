pub mod attachment_texture;
pub mod bindgroup;
pub mod buffer;
pub mod r#const;
pub mod pipeline;
pub mod sampler;
pub mod shader_util;
pub mod texture;
pub mod render_pass;

pub use attachment_texture::*;
pub use bindgroup::*;
pub use buffer::*;
pub use pipeline::*;
pub use sampler::*;
pub use texture::*;
pub use render_pass::*;

/// The renderer trait.
/// 
/// Impl this trait for build your own renderer.
pub trait Renderer: 'static + Sized{
  fn init(device: &wgpu::Device, size: (usize, usize)) -> Self;
  fn resize(&mut self, device: &wgpu::Device, size: (usize, usize));
  fn render();
}

/// WebGPU renderer backend
pub struct WGPURenderer<T: Renderer> {
  surface: wgpu::Surface,
  pub adapter: wgpu::Adapter,
  pub device: wgpu::Device,
  pub queue: wgpu::Queue,

  pub renderer: T,
  pub encoder: wgpu::CommandEncoder,

  pub swap_chain: wgpu::SwapChain,
  pub swap_chain_descriptor: wgpu::SwapChainDescriptor,
}

impl<T: Renderer> WGPURenderer<T> {
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
    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
    let renderer = T::init(&device, size);
    Self{
      surface,
      adapter,
      device,
      queue,
      encoder,
      renderer,

      swap_chain,
      swap_chain_descriptor,
    }
  }

  pub fn resize(&mut self, width: usize, height: usize) {
    self.swap_chain_descriptor.width = width as u32;
    self.swap_chain_descriptor.height = height as u32;
    self.swap_chain = self.device.create_swap_chain(&self.surface, &self.swap_chain_descriptor);
    self.renderer.resize(&self.device, (width, height))
  }

  pub fn create_buffer<D: 'static + Copy>(&self, data: &[D], usage: wgpu::BufferUsage) -> WGPUBuffer {
    WGPUBuffer::new(&self.device, &data, usage)
  }
  pub fn create_index_buffer<D: 'static + Copy>(&self, data: &[D]) -> WGPUBuffer {
    WGPUBuffer::new(&self.device, &data, wgpu::BufferUsage::INDEX)
  }
  pub fn create_vertex_buffer<D: 'static + Copy>(&self, data: &[D]) -> WGPUBuffer {
    WGPUBuffer::new(&self.device, &data, wgpu::BufferUsage::VERTEX)
  }


}
