use crate::*;

pub fn check_update_gpu_2d<'a, P>(
  source: &SceneTexture2D<P>,
  resources: &'a mut GPUResourceSubCache,
  gpu: &GPU,
) -> &'a GPUTexture2dView
where
  P: SceneContent,
  P::Texture2D: AsRef<dyn WebGPUTexture2dSource>,
{
  let texture = source.read();
  resources.texture_2ds.get_update_or_insert_with(
    &texture,
    |texture| {
      let texture = texture.as_ref();
      let desc = texture.create_tex2d_desc(MipLevelCount::EmptyMipMap);

      GPUTexture2d::create(desc, &gpu.device)
        .upload_into(&gpu.queue, texture, 0)
        .create_default_view()
    },
    |_, _| {},
  )
}

pub fn check_update_gpu_cube<'a, P>(
  source: &SceneTextureCube<P>,
  resources: &'a mut GPUResourceSubCache,
  gpu: &GPU,
) -> &'a GPUTextureCubeView
where
  P: SceneContent,
  P::TextureCube: AsRef<[Box<dyn WebGPUTexture2dSource>; 6]>,
{
  let texture = source.read();

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
