use rendiation_texture::CubeTextureFace;
use rendiation_webgpu::*;

use crate::*;

pub trait MaterialBindableResourceUpdate {
  fn update<'a>(
    &self,
    resources: &'a mut GPUResourceSubCache,
    device: &GPUDevice,
    queue: &wgpu::Queue,
  ) -> wgpu::BindingResource<'a>;
}

// impl GPUResourceSubCache {
//   pub fn update_texture(&mut self, tex: &SceneTexture2D, target: &mut GPUTexture2dView) {
//     //
//   }
// }

impl MaterialBindableResourceUpdate for SceneTexture2D {
  fn update<'a>(
    &self,
    resources: &'a mut GPUResourceSubCache,
    device: &GPUDevice,
    queue: &wgpu::Queue,
  ) -> wgpu::BindingResource<'a> {
    let texture = self.content.borrow();
    resources
      .texture_2ds
      .get_update_or_insert_with(
        &texture,
        |texture| {
          let source = texture.as_ref();
          let desc = source.create_tex2d_desc(MipLevelCount::EmptyMipMap);

          GPUTexture2d::create(desc, device).upload_into(queue, source, 0)
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
    device: &GPUDevice,
    queue: &wgpu::Queue,
  ) -> wgpu::BindingResource<'a> {
    let texture = self.content.borrow();

    resources
      .texture_cubes
      .get_update_or_insert_with(
        &texture,
        |texture| {
          let source = texture.as_ref();
          let desc = source[0].create_cube_desc(MipLevelCount::EmptyMipMap);

          GPUTextureCube::create(desc, device)
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
