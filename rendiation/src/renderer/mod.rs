pub mod attachment_texture;
pub mod bindgroup;
pub mod buffer;
pub mod r#const;
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
/// the render resource should be stored in injected trait renderer
pub struct WGPURenderer<'a> {
  surface: wgpu::Surface,
  pub adapter: wgpu::Adapter,
  pub device: wgpu::Device,
  pub queue: wgpu::Queue,
  pub encoder: wgpu::CommandEncoder,
  pub size: (usize, usize),

  pub target: Option<wgpu::SwapChainOutput<'a>>,
  pub swap_chain: wgpu::SwapChain,
  pub swap_chain_descriptor: wgpu::SwapChainDescriptor,
}

impl<'a> WGPURenderer<'a> {
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
    let target = None;
    Self {
      surface,
      adapter,
      device,
      queue,
      encoder,
      size,

      target,
      swap_chain,
      swap_chain_descriptor,
    }
  }

  pub fn resize(&mut self, size: (usize, usize)) {
    self.swap_chain_descriptor.width = size.0 as u32;
    self.swap_chain_descriptor.height = size.1 as u32;
    self.swap_chain = self
      .device
      .create_swap_chain(&self.surface, &self.swap_chain_descriptor);
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

  pub fn request_output(&'a mut self){
    self.target = Some(self.swap_chain.get_next_texture());
  }

  pub fn submit_queue(&mut self){
    let mut encoder = self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
    use std::mem;
    mem::swap(&mut self.encoder, &mut encoder);

    let command_buf = encoder.finish();
    self.queue.submit(&[command_buf]);
  }
}
