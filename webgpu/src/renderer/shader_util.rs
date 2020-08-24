use std::fmt::{Display};

#[cfg(feature = "glsl-to-spirv")]
pub fn load_glsl(code: impl AsRef<str> + Display, stage: wgpu::ShaderStage) -> Vec<u32> {
  print!("{}", code);
  let ty = match stage {
    wgpu::ShaderStage::VERTEX => glsl_to_spirv::ShaderType::Vertex,
    wgpu::ShaderStage::FRAGMENT => glsl_to_spirv::ShaderType::Fragment,
    wgpu::ShaderStage::COMPUTE => glsl_to_spirv::ShaderType::Compute,
    _ => panic!("unsupported"),
  };

  let spirv = glsl_to_spirv::compile(code.as_ref(), ty);
  if let Err(err) =  &spirv {
    print!("{}", code);
    println!("{}", err);
  }
  let spirv = wgpu::read_spirv(spirv.unwrap());
  if let Err(err) =  &spirv {
    print!("{}", code);
    println!("{}", err);
  }
  spirv.unwrap()
}
