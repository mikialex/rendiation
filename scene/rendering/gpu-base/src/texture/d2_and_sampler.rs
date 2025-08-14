use crate::*;

/// not need to hash the sampler to reduce the gpu sampler count, in device we have deduplicated
pub fn use_sampler_gpus(cx: &mut QueryGPUHookCx) -> (SharedHashMapRead<u32, GPUSamplerView>, bool) {
  let map = cx.use_shared_hash_map();

  let mut changed = false;

  maintain_shared_map(&map, cx.use_changes::<SceneSamplerInfo>(), |info| {
    changed = true;
    create_gpu_sampler(cx.gpu, &info)
  });

  (map.make_read_holder(), changed)
}

pub fn use_gpu_texture_2ds(
  cx: &mut QueryGPUHookCx,
  default: &GPU2DTextureView,
) -> (SharedHashMapRead<u32, GPU2DTextureView>, bool) {
  let map = cx.use_shared_hash_map();

  let mut changed = false;

  maintain_shared_map_avoid_unnecessary_creator_init(
    &map,
    cx.use_changes::<SceneTexture2dEntityDirectContent>(),
    || {
      changed = true;
      let mut mipmap_cx = MipmapCtx {
        gpu: cx.gpu.clone(),
        encoder: cx.gpu.create_encoder().into(),
      };

      move |tex| {
        if let Some(tex) = tex {
          create_gpu_texture2d_with_mipmap(
            &mipmap_cx.gpu,
            mipmap_cx.encoder.as_mut().unwrap(),
            &tex,
          )
        } else {
          default.clone()
        }
      }
    },
  );

  (map.make_read_holder(), changed)
}

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
