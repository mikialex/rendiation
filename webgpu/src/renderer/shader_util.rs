use std::{fmt::Display, io::Read};

use wgpu::ShaderStage;

#[cfg(feature = "glsl-to-spirv")]
#[allow(clippy::transmute_ptr_to_ptr)]
pub fn load_glsl(code: impl AsRef<str> + Display, stage: rendiation_ral::ShaderStage) -> Vec<u32> {
  // print!("{}", code);
  let ty = match stage {
    ShaderStage::VERTEX => glsl_to_spirv::ShaderType::Vertex,
    ShaderStage::FRAGMENT => glsl_to_spirv::ShaderType::Fragment,
    _ => unimplemented!(),
  };

  let spirv = glsl_to_spirv::compile(code.as_ref(), ty);
  if let Err(err) = &spirv {
    print!("{}", code);
    println!("{}", err);
  }

  let mut spirv_result = Vec::new();
  spirv.unwrap().read_to_end(&mut spirv_result).unwrap();

  let v = std::mem::ManuallyDrop::new(spirv_result);

  unsafe {
    let ptr = v.as_ptr();
    let ptr = std::mem::transmute(ptr);
    let size = v.len();
    let cap = v.capacity();

    Vec::from_raw_parts(ptr, size / 4, cap / 4)
  }
}
