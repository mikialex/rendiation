use rendiation_texture::GPUBufferImage;

use crate::*;

#[derive(Clone, Copy, Debug)]
pub struct BindableResourceConfig {
  /// decide if should enable texture bindless support if platform hardware supported
  pub prefer_bindless_texture: bool,
  /// decide if should enable mesh bindless (multi indirect draw) support if platform hardware
  /// supported
  pub prefer_bindless_mesh: bool,
}

pub fn make_texture_gpu_sys_default(
  gpu: &ResourceGPUCtx,
) -> GPUTextureBindingSystemDefaultResource {
  let default_texture_2d = GPUBufferImage {
    data: vec![255, 255, 255, 255],
    format: TextureFormat::Rgba8UnormSrgb,
    size: Size::from_u32_pair_min_one((1, 1)),
  };
  let default_texture_2d = SceneTexture2DType::GPUBufferImage(default_texture_2d);

  let default_sampler = Default::default();

  GPUTextureBindingSystemDefaultResource {
    texture: gpu.create_gpu_texture2d(&default_texture_2d),
    sampler: gpu.create_gpu_sampler(&default_sampler),
  }
}
