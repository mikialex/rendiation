use crate::*;

mod cube;
pub use cube::*;
use rendiation_texture_gpu_base::*;

/// not need to hash the sampler to reduce the gpu sampler count, in device we have deduplicated
/// already, and we also not need to do materialize, in device we have cached all sample created
pub fn sampler_gpus(cx: &GPU) -> impl ReactiveCollection<u32, GPUSamplerView> {
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
) -> impl ReactiveCollection<u32, GPU2DTextureView> {
  let cx = cx.clone();

  global_watch()
    .watch_untyped_key::<SceneTexture2dEntityDirectContent>()
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
