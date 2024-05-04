use rendiation_texture_core::TextureSampler;
use rendiation_texture_gpu_base::SamplerConvertExt;

use crate::*;

/// not need to hash the sampler to reduce the gpu sampler count, in device we have deduplicated
/// already, and we also not need to do materialize, in device we have cached all sample created
pub fn sampler_gpus(
  cx: &GPUResourceCtx,
) -> impl ReactiveCollection<AllocIdx<SceneSamplerEntity>, GPUSamplerView> {
  let cx = cx.clone();
  global_watch()
    .watch_typed_key::<SceneSamplerInfo>()
    // todo, we should consider using the simple map here
    .collective_execute_map_by(move || {
      let cx = cx.clone();
      move |_, s| create_gpu_sampler(&cx, &s)
    })
}

pub fn create_gpu_sampler(cx: &GPUResourceCtx, s: &TextureSampler) -> GPUSamplerView {
  let gpu_sampler = GPUSampler::create(s.into_gpu(), &cx.device);
  gpu_sampler.create_default_view()
}
