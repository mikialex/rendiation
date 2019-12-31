use crate::renderer::sampler::WGPUSampler;
use crate::renderer::texture::WGPUTexture;
use crate::renderer::buffer::WGPUBuffer;

pub enum WGPUBinding {
  BindBuffer(WGPUBuffer),
  BindTexture(wgpu::TextureView),
  BindSampler(WGPUSampler),
}

pub struct WGPUBindGroup {
  gpu_buffer: wgpu::BindGroup,
}

impl WGPUBindGroup{
  pub fn new(device: &wgpu::Device, bindings: &[WGPUBinding], layout: &wgpu::BindGroupLayout) -> Self{
    let resouce_wrap: Vec<_> = bindings.iter().map(|binding|{
      match binding{
        WGPUBinding::BindBuffer(buffer) =>{
          wgpu::BindingResource::Buffer {
            buffer: &buffer.get_gpu_buffer(),
            range: 0..buffer.get_byte_length() as u64,
          }
        },
        WGPUBinding::BindTexture(texture) => {
          wgpu::BindingResource::TextureView(&texture)
        },
        WGPUBinding::BindSampler(sampler) =>{
          wgpu::BindingResource::Sampler(sampler.get_gpu_sampler())
        }
      }
    }).collect();

    let mut count = 0;
    let wgpu_bindings: Vec<_> = resouce_wrap.iter().map(|resource|{
      let b = wgpu::Binding {
        binding: count,
        resource: resource.clone(), // todo improvement
      };
      count+= 1;
      b
    }).collect();

    let wgpu_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout,
      bindings: &wgpu_bindings,
    });

    Self{
      gpu_buffer:wgpu_bindgroup,
    }
  }

  // pub fn update() -> Self {
  //   Self{
  //     bindings: Vec::new()
  //   }
  // }

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