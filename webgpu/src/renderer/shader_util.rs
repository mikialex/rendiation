use std::fmt::Display;

#[cfg(feature = "glsl-to-spirv")]
pub fn load_glsl(code: impl AsRef<str> + Display, stage: rendiation_ral::ShaderStage) -> Vec<u32> {
  print!("{}", code);
  use rendiation_ral::ShaderStage::*;
  let ty = match stage {
    Vertex => glsl_to_spirv::ShaderType::Vertex,
    Fragment => glsl_to_spirv::ShaderType::Fragment,
    _ => panic!("unsupported"),
  };

  let spirv = glsl_to_spirv::compile(code.as_ref(), ty);
  if let Err(err) = &spirv {
    print!("{}", code);
    println!("{}", err);
  }
  let spirv = wgpu::read_spirv(spirv.unwrap());
  if let Err(err) = &spirv {
    print!("{}", code);
    println!("{}", err);
  }
  spirv.unwrap()
}
