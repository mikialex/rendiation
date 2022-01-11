use rendiation_texture::CubeTextureFace;
use rendiation_webgpu::*;

use crate::*;

pub trait MaterialBindableResourceUpdate {
  fn update<'a>(
    &self,
    resources: &'a mut GPUResourceSubCache,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
  ) -> wgpu::BindingResource<'a>;
}

impl MaterialBindableResourceUpdate for SceneTexture2D {
  fn update<'a>(
    &self,
    resources: &'a mut GPUResourceSubCache,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
  ) -> wgpu::BindingResource<'a> {
    let mut texture = self.content.borrow_mut();
    resources
      .texture_2ds
      .get_update_or_insert_with(
        &mut texture,
        |texture| {
          let source = texture.as_ref();
          let desc = source.create_tex2d_desc(MipLevelCount::EmptyMipMap);

          WebGPUTexture2d::create(device, desc).upload_into(queue, source, 0)
        },
        |_, _| {},
      )
      .as_bindable()
  }
}

impl MaterialBindableResourceUpdate for SceneTextureCube {
  fn update<'a>(
    &self,
    resources: &'a mut GPUResourceSubCache,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
  ) -> wgpu::BindingResource<'a> {
    let mut texture = self.content.borrow_mut();

    resources
      .texture_cubes
      .get_update_or_insert_with(
        &mut texture,
        |texture| {
          let source = texture.as_ref();
          let desc = source[0].create_cube_desc(MipLevelCount::EmptyMipMap);

          WebGPUTextureCube::create(device, desc)
            .upload(queue, source[0].as_ref(), CubeTextureFace::PositiveX, 0)
            .upload(queue, source[1].as_ref(), CubeTextureFace::NegativeX, 0)
            .upload(queue, source[2].as_ref(), CubeTextureFace::PositiveY, 0)
            .upload(queue, source[3].as_ref(), CubeTextureFace::NegativeY, 0)
            .upload(queue, source[4].as_ref(), CubeTextureFace::PositiveZ, 0)
            .upload(queue, source[5].as_ref(), CubeTextureFace::NegativeZ, 0)
        },
        |_, _| {},
      )
      .as_bindable()
  }
}
