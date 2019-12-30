pub enum WGPUBinding {
  WGPUBuffer,
  WGPUTexture,
  WGPUSampler,
}

pub struct WGPUBindGroup {
  gpu_buffer: wgpu::BindGroup,
  bindings: Vec<WGPUBinding>
}

impl WGPUBindGroup{
  pub fn new(device: &wgpu::Device, bindings: &[WGPUBinding], layout: &wgpu::BindGroupLayout) -> Self{
    let wgpu_bindings = bindings.iter().map(|binding|{
      let resource = match binding{
        WGPUBuffer =>{
          wgpu::BindingResource::Buffer {
            buffer: &uniform_buf.get_gpu_buffer(),
            range: 0..64,
          },
        }
        },
        WGPUTexture =>{
          wgpu::BindingResource::TextureView(&texture_view)
        },
        WGPUSampler =>{
          wgpu::BindingResource::Sampler(sampler.get_gpu_sampler())
        }
      }

    });

    let wgpu_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout,
      bindings: &wgpu_bindings,
    })

    Self{
      
    }
  }
  }


  pub fn update() -> Self {
    Self{
      bindings: Vec::new()
    }
  }

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

  pub fn build(){

  }
}