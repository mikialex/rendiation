#[cfg(feature = "glsl-to-spirv")]
pub fn load_glsl(code: impl AsRef<str>, stage: wgpu::ShaderStage) -> Vec<u32> {
  let ty = match stage {
    wgpu::ShaderStage::VERTEX => glsl_to_spirv::ShaderType::Vertex,
    wgpu::ShaderStage::FRAGMENT => glsl_to_spirv::ShaderType::Fragment,
    wgpu::ShaderStage::COMPUTE => glsl_to_spirv::ShaderType::Compute,
    _ => panic!("unsupported"),
  };

  wgpu::read_spirv(glsl_to_spirv::compile(code.as_ref(), ty).unwrap()).unwrap()
}
