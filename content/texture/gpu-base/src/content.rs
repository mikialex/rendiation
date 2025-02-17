use crate::*;

pub fn create_gpu_sampler(cx: &GPU, s: &TextureSampler) -> GPUSamplerView {
  let gpu_sampler = GPUSampler::create(s.into_gpu(), &cx.device);
  gpu_sampler.create_default_view()
}

pub fn create_gpu_texture2d(cx: &GPU, texture: &GPUBufferImage) -> GPU2DTextureView {
  let texture = GPUBufferImageForeignImpl { inner: texture };

  let desc = texture.create_tex2d_desc(MipLevelCount::BySize);
  let gpu_texture = GPUTexture::create(desc, &cx.device);
  let gpu_texture: GPU2DTexture = gpu_texture.try_into().unwrap();
  let gpu_texture = gpu_texture.upload_into(&cx.queue, &texture, 0);

  gpu_texture.create_default_view().try_into().unwrap()
}

pub fn create_gpu_texture2d_with_mipmap(
  cx: &GPU,
  encoder: &mut GPUCommandEncoder,
  texture: &GPUBufferImage,
) -> GPU2DTextureView {
  let texture = GPUBufferImageForeignImpl { inner: texture };

  let desc = texture.create_tex2d_desc(MipLevelCount::BySize);
  let gpu_texture = GPUTexture::create(desc, &cx.device);
  let gpu_texture: GPU2DTexture = gpu_texture.try_into().unwrap();
  let gpu_texture = gpu_texture.upload_into(&cx.queue, &texture, 0);

  DefaultMipmapReducer.generate(cx, encoder, &gpu_texture);

  gpu_texture.create_default_view().try_into().unwrap()
}

pub fn create_fallback_empty_texture(device: &GPUDevice) -> GPU2DTexture {
  GPUTexture::create(
    TextureDescriptor {
      label: "global default texture".into(),
      size: Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: TextureDimension::D2,
      format: TextureFormat::Rgba8Unorm,
      view_formats: &[],
      usage: basic_texture_usages(),
    },
    device,
  )
  .try_into()
  .unwrap()
}
