use rendiation_texture::CubeTextureFace;

use crate::*;

pub struct ReactiveGPUCubeTextureSignal {
  inner: EventSource<BindableGPUChange>,
  gpu: GPUCubeTextureView,
}

pub type TextureCubeRenderComponentDeltaStream = impl Stream<Item = RenderComponentDeltaFlag>;

pub type ReactiveGPUCubeTextureViewSource =
  impl AsRef<ReactiveGPUCubeTextureSignal> + Stream<Item = BindableGPUChange>;

impl ReactiveGPUCubeTextureSignal {
  pub fn create_gpu_texture_stream(&self) -> impl Stream<Item = BindableGPUChange> {
    let current = self.gpu.clone();
    self.inner.single_listen_by(
      |v| v.clone(),
      |send| send(BindableGPUChange::ReferenceCube(current)),
    )
  }
  pub fn create_gpu_texture_com_delta_stream(&self) -> TextureCubeRenderComponentDeltaStream {
    self
      .create_gpu_texture_stream()
      .map(BindableGPUChange::into_render_component_delta)
  }
}

impl ShareBindableResourceCtx {
  // todo cube texture bindless support
  pub fn get_or_create_reactive_gpu_texture_cube(
    &self,
    tex: &SceneTextureCube,
  ) -> (impl Stream<Item = BindableGPUChange>, GPUCubeTextureView) {
    let mut texture_cube = self.texture_cube.write().unwrap();
    let cache = texture_cube.get_or_insert_with(tex.guid(), || {
      let gpu_tex = self.gpu.create_gpu_texture_cube(tex);

      let gpu_tex = ReactiveGPUCubeTextureSignal {
        inner: Default::default(),
        gpu: gpu_tex,
      };

      let gpu_clone: ResourceGPUCtx = self.gpu.clone();

      tex
        .unbound_listen_by(any_change_no_init)
        .filter_map_sync(tex.defer_weak())
        .fold_signal(gpu_tex, move |tex, gpu_tex| {
          let recreated = gpu_clone.create_gpu_texture_cube(&tex);
          gpu_tex.gpu = recreated.clone();
          gpu_tex
            .inner
            .emit(&BindableGPUChange::ReferenceCube(gpu_tex.gpu.clone()));
          BindableGPUChange::ReferenceCube(recreated).into()
        })
    });

    let gpu = cache.as_ref().gpu.clone();
    let changes = cache.as_ref().create_gpu_texture_stream();
    (changes, gpu)
  }
}

impl ResourceGPUCtx {
  fn create_gpu_texture_cube(&self, tex: &SceneTextureCube) -> GPUCubeTextureView {
    let texture = tex.read();
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
