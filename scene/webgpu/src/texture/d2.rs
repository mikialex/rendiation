use crate::*;

pub fn gpu_texture_2ds(
  cx: &ResourceGPUCtx,
  scope: impl ReactiveCollection<AllocIdx<SceneTexture2DType>, ()>,
) -> impl ReactiveCollection<AllocIdx<SceneTexture2DType>, GPU2DTextureView> {
  let cx = cx.clone();
  storage_of::<SceneTexture2DType>()
    .listen_all_instance_changed_set()
    .filter_by_keyset(scope)
    .collective_execute_gpu_map(cx, |tex, cx| cx.create_gpu_texture2d(tex))
}

impl ResourceGPUCtx {
  fn create_gpu_texture2d(&self, texture: &SceneTexture2DType) -> GPU2DTextureView {
    let texture = as_2d_source(texture);

    let gpu_texture = if let Some(texture) = texture {
      let desc = texture.create_tex2d_desc(MipLevelCount::BySize);
      let gpu_texture = GPUTexture::create(desc, &self.device);
      let gpu_texture: GPU2DTexture = gpu_texture.try_into().unwrap();
      gpu_texture.upload_into(&self.queue, &texture, 0)
    } else {
      create_fallback_empty_texture(&self.device)
    };

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
