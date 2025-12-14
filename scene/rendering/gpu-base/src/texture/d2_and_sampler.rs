use crate::*;

/// not need to hash the sampler to reduce the gpu sampler count, in device we have deduplicated
pub fn use_sampler_gpus(cx: &mut QueryGPUHookCx) -> SharedHashMapRead<u32, GPUSamplerView> {
  let map = cx.use_shared_hash_map("sampler gpu mapping");

  maintain_shared_map(&map, cx.use_changes::<SceneSamplerInfo>(), |info| {
    create_gpu_sampler(cx.gpu, &info)
  });

  map.make_read_holder()
}

pub fn use_gpu_texture_2ds(
  cx: &mut QueryGPUHookCx,
  default: &GPU2DTextureView,
  source: UseResult<impl DataChanges<Key = u32, Value = Option<Arc<GPUBufferImage>>>>,
) -> SharedHashMapRead<u32, GPU2DTextureView> {
  let map = cx.use_shared_hash_map("texture2d gpu mapping");

  maintain_shared_map_avoid_unnecessary_creator_init(&map, source, || {
    let mut mipmap_cx = MipmapCtx {
      gpu: cx.gpu.clone(),
      encoder: cx.gpu.create_encoder().into(),
    };

    move |tex| {
      if let Some(tex) = tex {
        create_gpu_texture2d_with_mipmap(&mipmap_cx.gpu, mipmap_cx.encoder.as_mut().unwrap(), &tex)
      } else {
        default.clone()
      }
    }
  });

  map.make_read_holder()
}

struct MipmapCtx {
  gpu: GPU,
  encoder: Option<GPUCommandEncoder>,
}

impl Drop for MipmapCtx {
  fn drop(&mut self) {
    self.gpu.submit_encoder(self.encoder.take().unwrap());
  }
}
