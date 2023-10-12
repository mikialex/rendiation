use crate::*;

pub struct ReactiveGPUSamplerSignal {
  inner: EventSource<BindableGPUChange>,
  handle: SamplerHandle,
  gpu: GPUSamplerView,
}

pub type ReactiveGPUSamplerViewSource =
  impl AsRef<ReactiveGPUSamplerSignal> + Stream<Item = BindableGPUChange>;

impl ReactiveGPUSamplerSignal {
  pub fn create_gpu_sampler_stream(&self) -> impl Stream<Item = BindableGPUChange> {
    let current = self.gpu.clone();
    let handle = self.handle;
    self.inner.single_listen_by(
      |v| v.clone(),
      move |send| send(BindableGPUChange::ReferenceSampler(current, handle)),
    )
  }
}

impl ShareBindableResourceCtx {
  pub fn get_or_create_reactive_gpu_sampler(
    &self,
    sampler: &IncrementalSignalPtr<TextureSampler>,
  ) -> (impl Stream<Item = BindableGPUChange>, GPUSamplerView) {
    let mut samplers = self.sampler.write().unwrap();

    let cache = samplers.get_or_insert_with(sampler.guid(), || {
      let source = *sampler.read();

      let gpu_sampler = GPUSampler::create(source.into_gpu(), &self.gpu.device);
      let gpu_sampler = gpu_sampler.create_default_view();
      let handle = self.binding_sys.register_sampler(gpu_sampler.clone());

      let gpu_sampler = ReactiveGPUSamplerSignal {
        inner: Default::default(),
        gpu: gpu_sampler,
        handle,
      };

      let gpu_clone: ResourceGPUCtx = self.gpu.clone();
      let bs = self.binding_sys.clone();

      sampler
        .unbound_listen_by(any_change_no_init)
        .filter_map_sync(sampler.defer_weak())
        .fold_signal(gpu_sampler, move |sampler, gpu_tex| {
          let source = sampler.read();
          let source: TextureSampler = *source;
          // creation will cached in device side now
          // todo, apply this reuse in handle level
          let gpu_sampler = GPUSampler::create(source.into_gpu(), &gpu_clone.device);
          let recreated = gpu_sampler.create_default_view();

          gpu_tex.gpu = recreated.clone();
          bs.deregister_sampler(gpu_tex.handle);
          gpu_tex.handle = bs.register_sampler(gpu_tex.gpu.clone());

          gpu_tex.inner.emit(&BindableGPUChange::ReferenceSampler(
            gpu_tex.gpu.clone(),
            gpu_tex.handle,
          ));
          BindableGPUChange::ReferenceSampler(recreated, gpu_tex.handle).into()
        })
    });

    let gpu = cache.as_ref().gpu.clone();
    let changes = cache.as_ref().create_gpu_sampler_stream();
    (changes, gpu)
  }
}
