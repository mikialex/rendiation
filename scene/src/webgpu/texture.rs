use rendiation_texture::CubeTextureFace;
use rendiation_webgpu::*;

use crate::*;

impl SceneTexture2D<WebGPUScene> {
  pub fn check_update_gpu<'a>(
    &self,
    resources: &'a mut GPUResourceSubCache,
    gpu: &GPU,
  ) -> &'a GPUTexture2dView {
    let texture = self.content.borrow();
    resources.texture_2ds.get_update_or_insert_with(
      &texture,
      |texture| {
        let source = texture.as_ref();
        let desc = source.create_tex2d_desc(MipLevelCount::EmptyMipMap);

        GPUTexture2d::create(desc, &gpu.device)
          .upload_into(&gpu.queue, source, 0)
          .create_default_view()
      },
      |_, _| {},
    )
  }
}

impl SceneTextureCube<WebGPUScene> {
  pub fn check_update_gpu<'a>(
    &self,
    resources: &'a mut GPUResourceSubCache,
    gpu: &GPU,
  ) -> &'a GPUTextureCubeView {
    let texture = self.content.borrow();

    resources.texture_cubes.get_update_or_insert_with(
      &texture,
      |texture| {
        let source = texture.as_ref();
        let desc = source[0].create_cube_desc(MipLevelCount::EmptyMipMap);
        let queue = &gpu.queue;

        GPUTextureCube::create(desc, &gpu.device)
          .upload(queue, source[0].as_ref(), CubeTextureFace::PositiveX, 0)
          .upload(queue, source[1].as_ref(), CubeTextureFace::NegativeX, 0)
          .upload(queue, source[2].as_ref(), CubeTextureFace::PositiveY, 0)
          .upload(queue, source[3].as_ref(), CubeTextureFace::NegativeY, 0)
          .upload(queue, source[4].as_ref(), CubeTextureFace::PositiveZ, 0)
          .upload(queue, source[5].as_ref(), CubeTextureFace::NegativeZ, 0)
          .create_default_view()
      },
      |_, _| {},
    )
  }
}
