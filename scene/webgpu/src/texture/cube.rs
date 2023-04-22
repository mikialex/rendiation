use rendiation_texture::CubeTextureFace;

use crate::*;

pub struct ReactiveGPUCubeTextureSignal {
  inner: EventSource<TextureGPUChange>,
  gpu: GPUCubeTextureView,
}

#[pin_project::pin_project]
pub struct ReactiveGPUCubeTextureView {
  #[pin]
  pub changes: TextureCubeRenderComponentDeltaStream,
  pub gpu: GPUCubeTextureView,
}

impl Stream for ReactiveGPUCubeTextureView {
  type Item = RenderComponentDelta;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.changes.poll_next(cx)
  }
}
impl Deref for ReactiveGPUCubeTextureView {
  type Target = GPUCubeTextureView;
  fn deref(&self) -> &Self::Target {
    &self.gpu
  }
}

pub type TextureCubeRenderComponentDeltaStream = impl Stream<Item = RenderComponentDelta>;

pub type ReactiveGPUCubeTextureViewSource =
  impl AsRef<ReactiveGPUCubeTextureSignal> + Stream<Item = TextureGPUChange>;

impl ReactiveGPUCubeTextureSignal {
  // todo , fix send sync in webgpu resource first
  // pub fn create_gpu_texture_stream(&self) -> impl Stream<Item = TextureGPUChange> {
  //   // create channel here, and send the init value
  //   let s = self
  //     .inner
  //     .listen_by(TextureGPUChange::to_render_component_delta);

  //   s
  // }
  pub fn create_gpu_texture_com_delta_stream(&self) -> TextureCubeRenderComponentDeltaStream {
    self.inner.listen_by(
      TextureGPUChange::to_render_component_delta,
      RenderComponentDelta::ContentRef,
    )
  }
}

impl ShareBindableResourceCtx {
  pub fn get_or_create_reactive_gpu_texture_cube(
    &self,
    tex: &SceneTextureCube,
  ) -> ReactiveGPUCubeTextureView {
    let mut texture_cube = self.texture_cube.write().unwrap();
    let cache = texture_cube.get_or_insert_with(tex.id(), || {
      let gpu_tex = self.gpu.create_gpu_texture_cube(tex);

      let gpu_tex = ReactiveGPUCubeTextureSignal {
        inner: Default::default(),
        gpu: gpu_tex,
      };

      let gpu_clone: ResourceGPUCtx = self.gpu.clone();
      let weak_tex = tex.downgrade();

      tex
        .listen_by(any_change)
        .fold_signal(gpu_tex, move |_delta, gpu_tex| {
          if let Some(tex) = weak_tex.upgrade() {
            let recreated = gpu_clone.create_gpu_texture_cube(&tex);
            gpu_tex.gpu = recreated.clone();
            gpu_tex
              .inner
              .emit(&TextureGPUChange::ReferenceCube(gpu_tex.gpu.clone()));
            TextureGPUChange::ReferenceCube(recreated).into()
          } else {
            None
          }
        })
    });

    let gpu = cache.as_ref().gpu.clone();
    let changes = cache.as_ref().create_gpu_texture_com_delta_stream();
    ReactiveGPUCubeTextureView { changes, gpu }
  }
}

impl ResourceGPUCtx {
  fn create_gpu_texture_cube(&self, tex: &SceneTextureCube) -> GPUCubeTextureView {
    let texture = &tex.read();
    if let Some(t) = as_2d_source(&texture.faces[0]) {
      let source = &texture.faces;
      let desc = t.create_cube_desc(MipLevelCount::EmptyMipMap);
      let queue = &self.queue;

      let gpu_texture = GPUTexture::create(desc, &self.device);
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
      let tex: GPUCubeTexture = create_fallback_empty_cube_texture(&self.device);
      tex.create_default_view().try_into().unwrap()
    }
  }
}

fn create_fallback_empty_cube_texture(device: &GPUDevice) -> GPUCubeTexture {
  GPUTexture::create(
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
      format: webgpu::TextureFormat::Rgba8UnormSrgb,
      view_formats: &[],
      usage: webgpu::TextureUsages::all(),
    },
    device,
  )
  .try_into()
  .unwrap()
}
