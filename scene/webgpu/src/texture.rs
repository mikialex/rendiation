use crate::*;

pub struct GPUTextureSamplerPair {
  pub texture: GPU2DTextureView,
  pub sampler: GPUSamplerView,
}

impl GPUTextureSamplerPair {
  pub fn setup_pass(&self, ctx: &mut GPURenderPassCtx, group: impl Into<usize> + Copy) {
    ctx.binding.bind(&self.texture, group);
    ctx.binding.bind(&self.sampler, group);
  }

  pub fn uniform_and_sample(
    &self,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
    group: impl Into<usize> + Copy,
    position: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    let sampler = binding.uniform_by(&self.sampler, group);
    let texture = binding.uniform_by(&self.texture, group);
    texture.sample(sampler, position)
  }
}

pub fn build_texture_sampler_pair<S>(
  t: &Texture2DWithSamplingData<S>,
  gpu: &GPU,
  res: &mut GPUResourceSubCache,
) -> GPUTextureSamplerPair
where
  S: SceneContent,
  S::Texture2D: AsRef<dyn WebGPU2DTextureSource>,
{
  let sampler = GPUSampler::create(t.sampler.into(), &gpu.device);
  let sampler = sampler.create_default_view();
  let texture = check_update_gpu_2d::<S>(&t.texture, res, gpu).clone();
  GPUTextureSamplerPair { texture, sampler }
}

pub fn check_update_gpu_2d<'a, P>(
  source: &SceneTexture2D<P>,
  resources: &'a mut GPUResourceSubCache,
  gpu: &GPU,
) -> &'a GPU2DTextureView
where
  P: SceneContent,
  P::Texture2D: AsRef<dyn WebGPU2DTextureSource>,
{
  let texture = source.read();
  resources.texture_2ds.get_update_or_insert_with(
    &texture,
    |texture| {
      let texture = texture.as_ref();
      let desc = texture.create_tex2d_desc(MipLevelCount::EmptyMipMap);
      let gpu_texture = GPUTexture::create(desc, &gpu.device);
      let gpu_texture: GPU2DTexture = gpu_texture.try_into().unwrap();

      gpu_texture
        .upload_into(&gpu.queue, texture, 0)
        .create_default_view()
        .try_into()
        .unwrap()
    },
    |_, _| {},
  )
}

pub fn check_update_gpu_cube<'a, P>(
  source: &SceneTextureCube<P>,
  resources: &'a mut GPUResourceSubCache,
  gpu: &GPU,
) -> &'a GPUCubeTextureView
where
  P: SceneContent,
  P::TextureCube: AsRef<[Box<dyn WebGPU2DTextureSource>; 6]>,
{
  let texture = source.read();

  resources.texture_cubes.get_update_or_insert_with(
    &texture,
    |texture| {
      let source = texture.as_ref();
      let desc = source[0].create_cube_desc(MipLevelCount::EmptyMipMap);
      let queue = &gpu.queue;

      let gpu_texture = GPUTexture::create(desc, &gpu.device);
      let gpu_texture: GPUCubeTexture = gpu_texture.try_into().unwrap();

      gpu_texture
        .upload(queue, source[0].as_ref(), CubeTextureFace::PositiveX, 0)
        .upload(queue, source[1].as_ref(), CubeTextureFace::NegativeX, 0)
        .upload(queue, source[2].as_ref(), CubeTextureFace::PositiveY, 0)
        .upload(queue, source[3].as_ref(), CubeTextureFace::NegativeY, 0)
        .upload(queue, source[4].as_ref(), CubeTextureFace::PositiveZ, 0)
        .upload(queue, source[5].as_ref(), CubeTextureFace::NegativeZ, 0)
        .create_default_view()
        .try_into()
        .unwrap()
    },
    |_, _| {},
  )
}
