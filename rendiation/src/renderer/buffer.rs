pub struct WGPUBuffer {
  gpu_buffer: wgpu::Buffer,
  size: usize,
  usage:  wgpu::BufferUsage,
}

fn create_buffer(device: &wgpu::Device, value: &[f32], usage: wgpu::BufferUsage) -> wgpu::Buffer {
  device
    .create_buffer_mapped(
      value.len(),
      usage,
    )
    .fill_from_slice(value)
}

impl WGPUBuffer {
  pub fn new(device: &wgpu::Device, value: &[f32], usage: wgpu::BufferUsage) -> Self {
    Self {
      gpu_buffer: create_buffer(device, value, usage),
      size: value.len(),
      usage
    }
  }

  pub fn update(&self, device: &wgpu::Device, value: &[f32]) -> &Self {
    assert_eq!(self.size, value.len());

    self.gpu_buffer = create_buffer(device, value, self.usage);
    self
  }

  pub fn get_byte_length(&self) -> usize {
    self.size * 8
  }
}
