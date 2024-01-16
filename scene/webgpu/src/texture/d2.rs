use crate::*;

pub struct ReactiveGPU2DTextureSignal {
  inner: EventSource<BindableGPUChange>,
  handle: Texture2DHandle,
  gpu: GPU2DTextureView,
}

pub type ReactiveGPU2DTextureViewSource =
  impl AsRef<ReactiveGPU2DTextureSignal> + Stream<Item = BindableGPUChange>;

impl ReactiveGPU2DTextureSignal {
  pub fn create_gpu_texture_stream(&self) -> impl Stream<Item = BindableGPUChange> {
    let current = self.gpu.clone();
    let handle = self.handle;
    self.inner.single_listen_by(
      |v| v.clone(),
      move |send| send(BindableGPUChange::Reference2D(current, handle)),
    )
  }
}

impl ShareBindableResourceCtx {
  pub fn get_or_create_reactive_gpu_texture2d(
    &self,
    tex: &SceneTexture2D,
  ) -> (impl Stream<Item = BindableGPUChange>, GPU2DTextureView) {
    let mut texture_2d = self.texture_2d.write().unwrap();
    let cache = texture_2d.get_or_insert_with(tex.guid(), || {
      let gpu_tex = self.gpu.create_gpu_texture2d(tex);
      let handle = self.binding_sys.register_texture(gpu_tex.clone());

      let gpu_tex = ReactiveGPU2DTextureSignal {
        inner: Default::default(),
        gpu: gpu_tex,
        handle,
      };

      let gpu_clone: ResourceGPUCtx = self.gpu.clone();
      let bs = self.binding_sys.clone();

      tex
        .unbound_listen_by(any_change_no_init)
        .filter_map_sync(tex.defer_weak())
        .fold_signal(gpu_tex, move |tex, gpu_tex| {
          let recreated = gpu_clone.create_gpu_texture2d(&tex);

          gpu_tex.gpu = recreated.clone();
          bs.deregister_texture(gpu_tex.handle);
          gpu_tex.handle = bs.register_texture(gpu_tex.gpu.clone());

          gpu_tex.inner.emit(&BindableGPUChange::Reference2D(
            gpu_tex.gpu.clone(),
            gpu_tex.handle,
          ));
          BindableGPUChange::Reference2D(recreated, gpu_tex.handle).into()
        })
    });

    let gpu = cache.as_ref().gpu.clone();
    let changes = cache.as_ref().create_gpu_texture_stream();
    (changes, gpu)
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
      gpu_texture.upload_into(&self.queue, &texture, 0)
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

enum SceneTexture2DSourceImpl<'a> {
  GPUBufferImage(GPUBufferImageForeignImpl<'a>),
  Foreign(&'a dyn WebGPU2DTextureSource),
}

impl<'a> WebGPU2DTextureSource for SceneTexture2DSourceImpl<'a> {
  fn format(&self) -> TextureFormat {
    match self {
      SceneTexture2DSourceImpl::GPUBufferImage(t) => t.format(),
      SceneTexture2DSourceImpl::Foreign(t) => t.format(),
    }
  }

  fn as_bytes(&self) -> &[u8] {
    match self {
      SceneTexture2DSourceImpl::GPUBufferImage(t) => t.as_bytes(),
      SceneTexture2DSourceImpl::Foreign(t) => t.as_bytes(),
    }
  }

  fn size(&self) -> Size {
    match self {
      SceneTexture2DSourceImpl::GPUBufferImage(t) => t.size(),
      SceneTexture2DSourceImpl::Foreign(t) => t.size(),
    }
  }
}

pub fn as_2d_source(tex: &SceneTexture2DType) -> Option<impl WebGPU2DTextureSource + '_> {
  match tex {
    SceneTexture2DType::GPUBufferImage(tex) => Some(SceneTexture2DSourceImpl::GPUBufferImage(
      GPUBufferImageForeignImpl { inner: tex },
    )),
    SceneTexture2DType::Foreign(tex) => get_dyn_trait_downcaster_static!(WebGPU2DTextureSource)
      .downcast_ref(tex.as_ref().as_any())
      .map(SceneTexture2DSourceImpl::Foreign),
  }
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
