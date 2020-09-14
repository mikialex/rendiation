use crate::renderer::WGPURenderer;
use wgpu::util::DeviceExt;

pub struct WGPUBuffer {
  gpu_buffer: wgpu::Buffer,
  byte_size: usize,
  usage: wgpu::BufferUsage,
}

fn create_buffer(
  renderer: &WGPURenderer,
  contents: &[u8],
  usage: wgpu::BufferUsage,
) -> wgpu::Buffer {
  renderer
    .device
    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: None,
      contents,
      usage,
    })
}

impl WGPUBuffer {
  pub fn new(renderer: &WGPURenderer, value: &[u8], usage: wgpu::BufferUsage) -> Self {
    Self {
      gpu_buffer: create_buffer(renderer, value, usage),
      byte_size: value.len(),
      usage,
    }
  }

  pub fn byte_size(&self) -> usize {
    self.byte_size
  }

  pub fn usage(&self) -> wgpu::BufferUsage {
    self.usage
  }

  pub fn update(&mut self, renderer: &mut WGPURenderer, value: &[u8]) -> &Self {
    assert_eq!(self.byte_size, value.len());

    let new_gpu = create_buffer(renderer, value, wgpu::BufferUsage::COPY_SRC);
    renderer
      .encoder
      .copy_buffer_to_buffer(&new_gpu, 0, &self.gpu_buffer, 0, self.byte_size as u64);
    self
  }

  pub fn get_gpu_buffer(&self) -> &wgpu::Buffer {
    &self.gpu_buffer
  }
}
