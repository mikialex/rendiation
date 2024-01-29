use rendiation_texture::CubeTextureFace;

use crate::*;

pub fn gpu_texture_cubes(
  cx: ResourceGPUCtx,
  scope: impl ReactiveCollection<AllocIdx<SceneTextureCubeImpl>, ()>,
) -> impl ReactiveCollection<AllocIdx<SceneTextureCubeImpl>, GPUCubeTextureView> {
  storage_of::<SceneTextureCubeImpl>()
    .listen_all_instance_changed_set()
    .filter_by_keyset(scope)
    .collective_execute_gpu_map(cx, |tex, cx| cx.create_gpu_texture_cube(tex))
    .materialize_unordered()
}

impl ResourceGPUCtx {
  fn create_gpu_texture_cube(&self, texture: &SceneTextureCubeImpl) -> GPUCubeTextureView {
    let first = as_2d_source(&texture.faces[0]);

    let view_desc = TextureViewDescriptor {
      dimension: Some(TextureViewDimension::Cube),
      ..Default::default()
    };

    if let Some(t) = &first {
      let source = &texture.faces;
      let desc = t.create_cube_desc(MipLevelCount::EmptyMipMap);
      let queue = &self.queue;

      let gpu_texture = GPUTexture::create(desc, &self.device);
      let gpu_texture: GPUCubeTexture = gpu_texture.try_into().unwrap();

      #[rustfmt::skip]
      gpu_texture
        .upload(queue, &as_2d_source(&source[0]).unwrap(), CubeTextureFace::PositiveX, 0)
        .upload(queue, &as_2d_source(&source[1]).unwrap(), CubeTextureFace::NegativeX, 0)
        .upload(queue, &as_2d_source(&source[2]).unwrap(), CubeTextureFace::PositiveY, 0)
        .upload(queue, &as_2d_source(&source[3]).unwrap(), CubeTextureFace::NegativeY, 0)
        .upload(queue, &as_2d_source(&source[4]).unwrap(), CubeTextureFace::PositiveZ, 0)
        .upload(queue, &as_2d_source(&source[5]).unwrap(), CubeTextureFace::NegativeZ, 0)
        .create_view(view_desc)
        .try_into()
        .unwrap()
    } else {
      let tex: GPUCubeTexture = create_fallback_empty_cube_texture(&self.device);
      tex.create_view(view_desc).try_into().unwrap()
    }
  }
}

fn create_fallback_empty_cube_texture(device: &GPUDevice) -> GPUCubeTexture {
  GPUTexture::create(
    TextureDescriptor {
      label: "unimplemented default texture".into(),
      size: Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 6,
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
