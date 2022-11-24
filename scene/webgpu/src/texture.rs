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
    let texture = binding.uniform_by(&self.texture, group);
    let sampler = binding.uniform_by(&self.sampler, group);
    texture.sample(sampler, position)
  }
}

pub fn build_texture_sampler_pair(
  t: &Texture2DWithSamplingData,
  gpu: &GPU,
  res: &mut GPUResourceSubCache,
) -> GPUTextureSamplerPair {
  let sampler = GPUSampler::create(t.sampler.into(), &gpu.device);
  let sampler = sampler.create_default_view();
  let texture = check_update_gpu_2d(&t.texture, res, gpu).clone();
  GPUTextureSamplerPair { texture, sampler }
}

pub fn check_update_gpu_2d<'a>(
  source: &SceneTexture2D,
  resources: &'a mut GPUResourceSubCache,
  gpu: &GPU,
) -> &'a GPU2DTextureView {
  resources.texture_2ds.get_update_or_insert_with(
    &source.read(),
    |texture| {
      let texture: Option<&dyn WebGPU2DTextureSource> = match texture {
        SceneTexture2DType::RGBAu8(tex) => Some(tex),
        SceneTexture2DType::RGBu8(tex) => Some(tex),
        SceneTexture2DType::RGBAf32(tex) => Some(tex),
        SceneTexture2DType::Foreign(tex) => tex
          .as_any()
          .downcast_ref::<Box<dyn WebGPU2DTextureSource>>()
          .map(|t| t.as_ref()),
        _ => None,
      };

      let gpu_texture = if let Some(texture) = texture {
        let desc = texture.create_tex2d_desc(MipLevelCount::EmptyMipMap);
        let gpu_texture = GPUTexture::create(desc, &gpu.device);
        let gpu_texture: GPU2DTexture = gpu_texture.try_into().unwrap();
        let gpu_texture = gpu_texture.upload_into(&gpu.queue, texture, 0);
        gpu_texture
      } else {
        GPUTexture::create(
          webgpu::TextureDescriptor {
            label: todo!(),
            size: todo!(),
            mip_level_count: todo!(),
            sample_count: todo!(),
            dimension: todo!(),
            format: todo!(),
            usage: todo!(),
          },
          &gpu.device,
        )
        .try_into()
        .unwrap()
      };

      gpu_texture.create_default_view().try_into().unwrap()
    },
    |_, _| {},
  )
}

pub fn check_update_gpu_cube<'a>(
  source: &SceneTextureCube,
  resources: &'a mut GPUResourceSubCache,
  gpu: &GPU,
) -> &'a GPUCubeTextureView {
  todo!()
  // let texture = source.read();

  // resources.texture_cubes.get_update_or_insert_with(
  //   &texture,
  //   |texture| {
  //     let source = texture.as_ref();
  //     let desc = source[0].create_cube_desc(MipLevelCount::EmptyMipMap);
  //     let queue = &gpu.queue;

  //     let gpu_texture = GPUTexture::create(desc, &gpu.device);
  //     let gpu_texture: GPUCubeTexture = gpu_texture.try_into().unwrap();

  //     gpu_texture
  //       .upload(queue, source[0].as_ref(), CubeTextureFace::PositiveX, 0)
  //       .upload(queue, source[1].as_ref(), CubeTextureFace::NegativeX, 0)
  //       .upload(queue, source[2].as_ref(), CubeTextureFace::PositiveY, 0)
  //       .upload(queue, source[3].as_ref(), CubeTextureFace::NegativeY, 0)
  //       .upload(queue, source[4].as_ref(), CubeTextureFace::PositiveZ, 0)
  //       .upload(queue, source[5].as_ref(), CubeTextureFace::NegativeZ, 0)
  //       .create_default_view()
  //       .try_into()
  //       .unwrap()
  //   },
  //   |_, _| {},
  // )
}
