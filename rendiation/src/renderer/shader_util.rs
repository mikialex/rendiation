#[allow(dead_code)]
pub enum ShaderType {
  Vertex,
  Fragment,
  Compute,
}

impl ShaderType {
  pub fn to_wgpu(&self) -> wgpu::ShaderStage {
    match self {
      ShaderType::Fragment => wgpu::ShaderStage::FRAGMENT,
      ShaderType::Vertex => wgpu::ShaderStage::VERTEX,
      ShaderType::Compute => wgpu::ShaderStage::COMPUTE,
    }
  }
}

#[cfg(feature = "glsl-to-spirv")]
pub fn load_glsl(code: &str, stage: ShaderType) -> Vec<u32> {
  let ty = match stage {
    ShaderType::Vertex => glsl_to_spirv::ShaderType::Vertex,
    ShaderType::Fragment => glsl_to_spirv::ShaderType::Fragment,
    ShaderType::Compute => glsl_to_spirv::ShaderType::Compute,
  };

  wgpu::read_spirv(glsl_to_spirv::compile(&code, ty).unwrap()).unwrap()
}
