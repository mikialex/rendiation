use crate::*;

/// not need to hash the sampler to reduce the gpu sampler count, in device we have deduplicated
/// already, and we also not need to do materialize, in device we have cached all sample created
pub fn sampler_gpus(
  cx: &ResourceGPUCtx,
) -> impl ReactiveCollection<AllocIdx<TextureSampler>, GPUSamplerView> {
  let cx = cx.clone();
  storage_of::<TextureSampler>()
    .listen_all_instance_changed_set()
    .collective_execute_map_by(move || {
      let cx = cx.clone();
      let creator = storage_of::<TextureSampler>().create_key_mapper(move |source, _| {
        let cx = cx.clone();
        let gpu_sampler = GPUSampler::create(source.into_gpu(), &cx.device);
        gpu_sampler.create_default_view()
      });
      move |k, _| creator(*k)
    })
}
