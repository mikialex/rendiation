use crate::*;

pub struct ReactiveGPUSamplerSignal {
  inner: EventSource<TextureGPUChange>,
  gpu: GPUSamplerView,
}

#[pin_project::pin_project]
pub struct ReactiveGPUSamplerView {
  #[pin]
  pub changes: SamplerRenderComponentDeltaStream,
  pub gpu: GPUSamplerView,
}

impl Stream for ReactiveGPUSamplerView {
  type Item = RenderComponentDeltaFlag;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    this.changes.poll_next(cx)
  }
}
impl Deref for ReactiveGPUSamplerView {
  type Target = GPUSamplerView;
  fn deref(&self) -> &Self::Target {
    &self.gpu
  }
}

pub type SamplerRenderComponentDeltaStream = impl Stream<Item = RenderComponentDeltaFlag>;

pub type ReactiveGPUSamplerViewSource =
  impl AsRef<ReactiveGPUSamplerSignal> + Stream<Item = TextureGPUChange>;

impl ReactiveGPUSamplerSignal {
  // todo , fix send sync in webgpu resource first
  // pub fn create_gpu_texture_stream(&self) -> impl Stream<Item = TextureGPUChange> {
  //   // create channel here, and send the init value
  //   let s = self
  //     .inner
  //     .listen_by(TextureGPUChange::to_render_component_delta);

  //   s
  // }
  pub fn create_gpu_sampler_com_delta_stream(&self) -> SamplerRenderComponentDeltaStream {
    self
      .inner
      .unbound_listen_by(TextureGPUChange::to_render_component_delta, |v| {
        v(RenderComponentDeltaFlag::ContentRef)
      })
  }
}

impl ShareBindableResourceCtx {
  pub fn get_or_create_reactive_gpu_sampler(
    &self,
    sampler: &SceneItemRef<TextureSampler>,
  ) -> ReactiveGPUSamplerView {
    let mut samplers = self.sampler.write().unwrap();

    let cache = samplers.get_or_insert_with(sampler.guid(), || {
      let source = sampler.read();
      let source: TextureSampler = **source;

      let gpu_sampler = GPUSampler::create(source.into(), &self.gpu.device);
      let gpu_sampler = gpu_sampler.create_default_view();

      let gpu_sampler = ReactiveGPUSamplerSignal {
        inner: Default::default(),
        gpu: gpu_sampler,
      };

      let gpu_clone: ResourceGPUCtx = self.gpu.clone();

      sampler
        .unbound_listen_by(any_change_no_init)
        .filter_map_sync(sampler.defer_weak())
        .fold_signal(gpu_sampler, move |sampler, gpu_tex| {
          let source = sampler.read();
          let source: TextureSampler = **source;
          // creation will cached in device side now
          let gpu_sampler = GPUSampler::create(source.into(), &gpu_clone.device);
          let recreated = gpu_sampler.create_default_view();

          gpu_tex.gpu = recreated.clone();
          gpu_tex
            .inner
            .emit(&TextureGPUChange::ReferenceSampler(gpu_tex.gpu.clone()));
          TextureGPUChange::ReferenceSampler(recreated).into()
        })
    });

    let gpu = cache.as_ref().gpu.clone();
    let changes = cache.as_ref().create_gpu_sampler_com_delta_stream();
    ReactiveGPUSamplerView { changes, gpu }
  }
}
