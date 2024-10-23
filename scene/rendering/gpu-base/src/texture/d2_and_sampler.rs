use crate::*;

/// not need to hash the sampler to reduce the gpu sampler count, in device we have deduplicated
/// already, and we also not need to do materialize, in device we have cached all sample created
pub fn sampler_gpus(cx: &GPU) -> impl ReactiveQuery<Key = u32, Value = GPUSamplerView> {
  let cx = cx.clone();
  global_watch()
    .watch_untyped_key::<SceneSamplerInfo>()
    // todo, we should consider using the simple map here
    .collective_execute_map_by(move || {
      let cx = cx.clone();
      move |_, s| create_gpu_sampler(&cx, &s)
    })
}

pub fn gpu_texture_2ds(
  cx: &GPU,
  default: GPU2DTextureView,
) -> impl ReactiveQuery<Key = u32, Value = GPU2DTextureView> {
  let cx = cx.clone();

  global_watch()
    .watch_untyped_key::<SceneTexture2dEntityDirectContent>()
    .collective_execute_map_by(move || {
      let cx = cx.clone();
      let default = default.clone();
      let mut cx = MipmapCtx {
        gpu: cx.clone(),
        encoder: cx.create_encoder().into(),
      };
      move |_, tex| {
        let cx = &mut cx;
        tex
          .map(move |tex| {
            create_gpu_texture2d_with_mipmap(&cx.gpu, cx.encoder.as_mut().unwrap(), &tex)
          })
          .unwrap_or_else(|| default.clone())
      }
    })
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
