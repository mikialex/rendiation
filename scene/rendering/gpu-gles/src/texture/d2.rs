use rendiation_texture::GPUBufferImage;
use rendiation_texture_gpu_base::GPUBufferImageForeignImpl;

use crate::*;

pub fn gpu_texture_2ds(
  cx: &GPUResourceCtx,
) -> impl ReactiveCollection<AllocIdx<SceneTexture2dEntity>, GPU2DTextureView> {
  let cx = cx.clone();
  let default: GPU2DTextureView = create_fallback_empty_texture(&cx.device)
    .create_default_view()
    .try_into()
    .unwrap();

  global_watch()
    .watch_typed_key::<SceneTexture2dEntityDirectContent>()
    // todo, we should consider using the simple map here
    .collective_execute_map_by(move || {
      let cx = cx.clone();
      let default = default.clone();
      move |_, tex| {
        tex
          .map(|tex| create_gpu_texture2d(&cx, &tex))
          .unwrap_or_else(|| default.clone())
      }
    })
}

pub fn create_gpu_texture2d(cx: &GPUResourceCtx, texture: &GPUBufferImage) -> GPU2DTextureView {
  let texture = GPUBufferImageForeignImpl { inner: texture };

  let desc = texture.create_tex2d_desc(MipLevelCount::BySize);
  let gpu_texture = GPUTexture::create(desc, &cx.device);
  let gpu_texture: GPU2DTexture = gpu_texture.try_into().unwrap();
  let gpu_texture = gpu_texture.upload_into(&cx.queue, &texture, 0);

  gpu_texture.create_default_view().try_into().unwrap()
}

fn create_fallback_empty_texture(device: &GPUDevice) -> GPU2DTexture {
  GPUTexture::create(
    TextureDescriptor {
      label: "unimplemented default texture".into(),
      size: Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: TextureDimension::D2,
      format: TextureFormat::Rgba8UnormSrgb,
      view_formats: &[],
      usage: TextureUsages::all(),
    },
    device,
  )
  .try_into()
  .unwrap()
}
