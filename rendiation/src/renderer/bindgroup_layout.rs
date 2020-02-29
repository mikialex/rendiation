use crate::renderer::shader_util::ShaderType;

pub struct BindGroupLayoutBuilder {
  pub bindings: Vec<wgpu::BindGroupLayoutBinding>,
}

fn shader_stage_convert(s: ShaderType) -> wgpu::ShaderStage {
  match s {
    ShaderType::Fragment => wgpu::ShaderStage::FRAGMENT,
    ShaderType::Vertex => wgpu::ShaderStage::VERTEX,
    _ => panic!()
  }
}

impl BindGroupLayoutBuilder {
  pub fn new() -> Self {
    Self {
      bindings: Vec::new(),
    }
  }

  pub fn bind_uniform_buffer(mut self, visibility: ShaderType) -> Self {
    let bindpoint = self.bindings.len() as u32;
    let visibility = shader_stage_convert(visibility);
    self.bindings.push(wgpu::BindGroupLayoutBinding {
      binding: bindpoint,
      visibility,
      ty: wgpu::BindingType::UniformBuffer { dynamic: false },
    });
    self
  }

  pub fn bind_texture2d(mut self, visibility: ShaderType) -> Self {
    let bindpoint = self.bindings.len() as u32;
    let visibility = shader_stage_convert(visibility);
    self.bindings.push(wgpu::BindGroupLayoutBinding {
      binding: bindpoint,
      visibility,
      ty: wgpu::BindingType::SampledTexture {
        multisampled: false,
        dimension: wgpu::TextureViewDimension::D2,
      },
    });
    self
  }

  pub fn bind_sampler(mut self, visibility: ShaderType) -> Self {
    let bindpoint = self.bindings.len() as u32;
    let visibility = shader_stage_convert(visibility);
    self.bindings.push(wgpu::BindGroupLayoutBinding {
      binding: bindpoint,
      visibility,
      ty: wgpu::BindingType::Sampler,
    });
    self
  }
}
