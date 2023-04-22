use crate::*;

pub struct ReactiveGPU2DTextureSignal {
  inner: EventSource<TextureGPUChange>,
  gpu: GPU2DTextureView,
}

#[pin_project::pin_project]
pub struct ReactiveGPU2DTextureView {
  #[pin]
  pub changes: Texture2dRenderComponentDeltaStream,
  pub gpu: GPU2DTextureView,
}

impl Stream for ReactiveGPU2DTextureView {
  type Item = RenderComponentDelta;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.changes.poll_next(cx)
  }
}
impl Deref for ReactiveGPU2DTextureView {
  type Target = GPU2DTextureView;
  fn deref(&self) -> &Self::Target {
    &self.gpu
  }
}

pub type Texture2dRenderComponentDeltaStream = impl Stream<Item = RenderComponentDelta>;

pub type ReactiveGPU2DTextureViewSource =
  impl AsRef<ReactiveGPU2DTextureSignal> + Stream<Item = TextureGPUChange>;

impl ReactiveGPU2DTextureSignal {
  // todo , fix send sync in webgpu resource first
  // pub fn create_gpu_texture_stream(&self) -> impl Stream<Item = TextureGPUChange> {
  //   // create channel here, and send the init value
  //   let s = self
  //     .inner
  //     .listen_by(TextureGPUChange::to_render_component_delta);

  //   s
  // }
  pub fn create_gpu_texture_com_delta_stream(&self) -> Texture2dRenderComponentDeltaStream {
    self.inner.listen_by(
      TextureGPUChange::to_render_component_delta,
      RenderComponentDelta::ContentRef,
    )
  }
}

impl ShareBindableResourceCtx {
  pub fn get_or_create_reactive_gpu_texture2d(
    &self,
    tex: &SceneTexture2D,
  ) -> ReactiveGPU2DTextureView {
    let mut texture_2d = self.texture_2d.write().unwrap();
    let cache = texture_2d.get_or_insert_with(tex.id(), || {
      let gpu_tex = self.gpu.create_gpu_texture2d(tex);

      let gpu_tex = ReactiveGPU2DTextureSignal {
        inner: Default::default(),
        gpu: gpu_tex,
      };

      let gpu_clone: ResourceGPUCtx = self.gpu.clone();
      let weak_tex = tex.downgrade();

      tex
        .listen_by(any_change)
        .fold_signal(gpu_tex, move |_delta, gpu_tex| {
          if let Some(tex) = weak_tex.upgrade() {
            let recreated = gpu_clone.create_gpu_texture2d(&tex);
            gpu_tex.gpu = recreated.clone();
            gpu_tex
              .inner
              .emit(&TextureGPUChange::Reference2D(gpu_tex.gpu.clone()));
            TextureGPUChange::Reference2D(recreated).into()
          } else {
            None
          }
        })
    });

    let gpu = cache.as_ref().gpu.clone();
    let changes = cache.as_ref().create_gpu_texture_com_delta_stream();
    ReactiveGPU2DTextureView { changes, gpu }
  }
}

impl ResourceGPUCtx {
  fn create_gpu_texture2d(&self, tex: &SceneTexture2D) -> GPU2DTextureView {
    let texture = &tex.read();
    let texture = as_2d_source(texture);

    let gpu_texture = if let Some(texture) = texture {
      let desc = texture.create_tex2d_desc(MipLevelCount::BySize);
      let gpu_texture = GPUTexture::create(desc, &self.device);
      let gpu_texture: GPU2DTexture = gpu_texture.try_into().unwrap();
      gpu_texture.upload_into(&self.queue, texture, 0)
    } else {
      create_fallback_empty_texture(&self.device)
    };

    self
      .mipmap_gen
      .borrow_mut()
      .request_mipmap_gen(&gpu_texture);
    gpu_texture.create_default_view().try_into().unwrap()
  }
}

pub fn as_2d_source(tex: &SceneTexture2DType) -> Option<&dyn WebGPU2DTextureSource> {
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

fn create_fallback_empty_texture(device: &GPUDevice) -> GPU2DTexture {
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
      format: webgpu::TextureFormat::Rgba8UnormSrgb,
      view_formats: &[],
      usage: webgpu::TextureUsages::all(),
    },
    device,
  )
  .try_into()
  .unwrap()
}
