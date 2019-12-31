use crate::renderer::buffer::WGPUBuffer;
use crate::renderer::sampler::WGPUSampler;

pub enum WGPUBinding<'a> {
  BindBuffer(&'a WGPUBuffer),
  BindTexture(&'a wgpu::TextureView),
  BindSampler(&'a WGPUSampler),
}

pub struct WGPUBindGroup {
  pub gpu_bindgroup: wgpu::BindGroup,
}

impl WGPUBindGroup {
  pub fn new(
    device: &wgpu::Device,
    bindings: &[WGPUBinding],
    layout: &wgpu::BindGroupLayout,
  ) -> Self {
    let resouce_wrap: Vec<_> = bindings
      .iter()
      .map(|binding| match binding {
        WGPUBinding::BindBuffer(buffer) => wgpu::BindingResource::Buffer {
          buffer: &buffer.get_gpu_buffer(),
          range: 0..buffer.get_byte_length() as u64,
        },
        WGPUBinding::BindTexture(texture) => wgpu::BindingResource::TextureView(&texture),
        WGPUBinding::BindSampler(sampler) => {
          wgpu::BindingResource::Sampler(sampler.get_gpu_sampler())
        }
      })
      .collect();

    let mut count = 0;
    let wgpu_bindings: Vec<_> = resouce_wrap
      .iter()
      .map(|resource| {
        let b = wgpu::Binding {
          binding: count,
          resource: resource.clone(), // todo improvement
        };
        count += 1;
        b
      })
      .collect();

    let wgpu_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout,
      bindings: &wgpu_bindings,
    });

    Self {
      gpu_bindgroup: wgpu_bindgroup,
    }
  }

  // pub fn update() -> Self {
  //   Self{
  //     bindings: Vec::new()
  //   }
  // }
}

pub struct BindGroupBuilder<'a> {
  pub bindings: Vec<WGPUBinding<'a>>,
}

impl<'a> BindGroupBuilder<'a> {
  pub fn new() -> Self {
    Self {
      bindings: Vec::new(),
    }
  }

  pub fn buffer(mut self, b: &'a WGPUBuffer) -> Self {
    self.bindings.push(WGPUBinding::BindBuffer(b));
    self
  }

  pub fn texture(mut self, t: &'a wgpu::TextureView) -> Self {
    self.bindings.push(WGPUBinding::BindTexture(t));
    self
  }

  pub fn sampler(mut self, s: &'a WGPUSampler) -> Self {
    self.bindings.push(WGPUBinding::BindSampler(s));
    self
  }

  pub fn build(&self, device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> WGPUBindGroup {
    WGPUBindGroup::new(device, &self.bindings, layout)
  }
}
