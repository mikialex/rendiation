use crate::renderer::shader_util::ShaderType;

pub struct BindGroupLayoutBuilder {
  pub bindings: Vec<wgpu::BindGroupLayoutBinding>,
}

impl BindGroupLayoutBuilder {
  pub fn new() -> Self {
    Self {
      bindings: Vec::new(),
    }
  }

  pub fn bind_uniform_buffer(mut self, visibility: ShaderType) -> Self {
    let bindpoint = self.bindings.len() as u32;
    self.bindings.push(wgpu::BindGroupLayoutBinding {
      binding: bindpoint,
      visibility: visibility.to_wgpu(),
      ty: wgpu::BindingType::UniformBuffer { dynamic: false },
    });
    self
  }

  pub fn bind_texture2d(mut self, visibility: ShaderType) -> Self {
    let bindpoint = self.bindings.len() as u32;
    self.bindings.push(wgpu::BindGroupLayoutBinding {
      binding: bindpoint,
      visibility: visibility.to_wgpu(),
      ty: wgpu::BindingType::SampledTexture {
        multisampled: false,
        dimension: wgpu::TextureViewDimension::D2,
      },
    });
    self
  }

  pub fn bind_sampler(mut self, visibility: ShaderType) -> Self {
    let bindpoint = self.bindings.len() as u32;
    self.bindings.push(wgpu::BindGroupLayoutBinding {
      binding: bindpoint,
      visibility: visibility.to_wgpu(),
      ty: wgpu::BindingType::Sampler,
    });
    self
  }
}
