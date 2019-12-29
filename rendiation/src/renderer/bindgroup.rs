pub struct WGPUBindGroup {
  gpu_buffer: wgpu::BindGroup,
}


pub struct BindGroupBuilder{
  pub bindings: Vec<wgpu::BindGroupLayoutBinding>,
}

impl BindGroupBuilder {
  pub fn new() -> Self {
    Self{
      bindings: Vec::new()
    }
  }

  pub fn binding(mut self, b: wgpu::BindGroupLayoutBinding) -> Self {
    self.bindings.push(b);
    self
  }

  // pub fn 
}