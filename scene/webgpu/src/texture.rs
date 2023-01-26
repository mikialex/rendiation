use rendiation_texture::CubeTextureFace;

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

fn as_2d_source(tex: &SceneTexture2DType) -> Option<&dyn WebGPU2DTextureSource> {
  match tex {
    SceneTexture2DType::RGBAu8(tex) => Some(tex),
    SceneTexture2DType::RGBu8(tex) => Some(tex),
    SceneTexture2DType::RGBAf32(tex) => Some(tex),
    SceneTexture2DType::Foreign(tex) => tex
      .downcast_ref::<Box<dyn WebGPU2DTextureSource>>()
      .map(|t| t.as_ref()),
    _ => None,
  }
}

pub fn check_update_gpu_2d<'a>(
  source: &SceneTexture2D,
  resources: &'a mut GPUResourceSubCache,
  gpu: &GPU,
) -> &'a GPU2DTextureView {
  resources.texture_2ds.get_update_or_insert_with(
    &source.read(),
    |texture| {
      let texture = as_2d_source(texture);

      let gpu_texture = if let Some(texture) = texture {
        let desc = texture.create_tex2d_desc(MipLevelCount::BySize);
        let gpu_texture = GPUTexture::create(desc, &gpu.device);
        let gpu_texture: GPU2DTexture = gpu_texture.try_into().unwrap();
        gpu_texture.upload_into(&gpu.queue, texture, 0)
      } else {
        GPUTexture::create(
          webgpu::TextureDescriptor {
            label: "unimplemented default texture".into(),
            size: webgpu::Extent3d {
              width: 1,
              height: 1,
              depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: webgpu::TextureDimension::D2,
            format: webgpu::TextureFormat::Rgba8Unorm,
            view_formats: &[],
            usage: webgpu::TextureUsages::all(),
          },
          &gpu.device,
        )
        .try_into()
        .unwrap()
      };

      resources
        .mipmap_gen
        .borrow_mut()
        .request_mipmap_gen(&gpu_texture);

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
  let texture = source.read();

  resources.texture_cubes.get_update_or_insert_with(
    &texture,
    |texture| {
      if let Some(t) = as_2d_source(&texture.faces[0]) {
        let source = &texture.faces;
        let desc = t.create_cube_desc(MipLevelCount::EmptyMipMap);
        let queue = &gpu.queue;

        let gpu_texture = GPUTexture::create(desc, &gpu.device);
        let gpu_texture: GPUCubeTexture = gpu_texture.try_into().unwrap();

        #[rustfmt::skip]
        gpu_texture
          .upload(queue, as_2d_source(&source[0]).unwrap(), CubeTextureFace::PositiveX, 0)
          .upload(queue, as_2d_source(&source[1]).unwrap(), CubeTextureFace::NegativeX, 0)
          .upload(queue, as_2d_source(&source[2]).unwrap(), CubeTextureFace::PositiveY, 0)
          .upload(queue, as_2d_source(&source[3]).unwrap(), CubeTextureFace::NegativeY, 0)
          .upload(queue, as_2d_source(&source[4]).unwrap(), CubeTextureFace::PositiveZ, 0)
          .upload(queue, as_2d_source(&source[5]).unwrap(), CubeTextureFace::NegativeZ, 0)
          .create_default_view()
          .try_into()
          .unwrap()
      } else {
        let tex: GPUCubeTexture = GPUTexture::create(
          webgpu::TextureDescriptor {
            label: "unimplemented default texture".into(),
            size: webgpu::Extent3d {
              width: 1,
              height: 1,
              depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: webgpu::TextureDimension::D2,
            format: webgpu::TextureFormat::Rgba8Unorm,
            view_formats: &[],
            usage: webgpu::TextureUsages::all(),
          },
          &gpu.device,
        )
        .try_into()
        .unwrap();
        tex.create_default_view().try_into().unwrap()
      }
    },
    |_, _| {},
  )
}
