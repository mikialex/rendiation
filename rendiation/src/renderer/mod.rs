pub mod bindgroup;
pub mod buffer;
pub mod consts;
pub mod pipeline;
pub mod bindgroup_layout;
pub mod render_pass;
pub mod sampler;
pub mod shader_util;
pub mod texture;
pub mod swap_chain;
pub mod pipeline_builder;

pub use bindgroup::*;
pub use buffer::*;
pub use pipeline::*;
pub use bindgroup_layout::*;
pub use render_pass::*;
pub use sampler::*;
pub use texture::*;
pub use consts::*;
pub use shader_util::*;
pub use swap_chain::*;

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
  pub adapter: wgpu::Adapter,
  pub device: wgpu::Device,
  pub queue: Queue,
  pub encoder: wgpu::CommandEncoder,
  pub swap_chain_format: wgpu::TextureFormat,
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

impl WGPURenderer {
  pub fn new() -> Self {
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
    Self {
      adapter,
      device,
      queue: Queue(queue),
      encoder,
      swap_chain_format: wgpu::TextureFormat::Bgra8UnormSrgb,
    }
  }

  // pub fn create_buffer<D: 'static + Copy>(
  //   &self,
  //   data: &[D],
  //   usage: wgpu::BufferUsage,
  // ) -> WGPUBuffer {
  //   WGPUBuffer::new(&self.device, &data, usage)
  // }
  // pub fn create_index_buffer<D: 'static + Copy>(&self, data: &[D]) -> WGPUBuffer {
  //   WGPUBuffer::new(&self.device, &data, wgpu::BufferUsage::INDEX)
  // }
  // pub fn create_vertex_buffer<D: 'static + Copy>(&self, data: &[D]) -> WGPUBuffer {
  //   WGPUBuffer::new(&self.device, &data, wgpu::BufferUsage::VERTEX)
  // }

}
