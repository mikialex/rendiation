use crate::*;

mod cube;
mod d2;
mod pair;
mod sampler;

pub use cube::*;
pub use d2::*;
pub use pair::*;
pub use sampler::*;

#[derive(Clone)]
pub enum BindableGPUChange {
  Reference2D(GPU2DTextureView, Texture2DHandle),
  ReferenceCube(GPUCubeTextureView),
  ReferenceSampler(GPUSamplerView, SamplerHandle),
  Content,
}

impl BindableGPUChange {
  fn into_render_component_delta(self) -> RenderComponentDeltaFlag {
    match self {
      BindableGPUChange::Reference2D(..) => RenderComponentDeltaFlag::ContentRef,
      BindableGPUChange::ReferenceCube(..) => RenderComponentDeltaFlag::ContentRef,
      BindableGPUChange::ReferenceSampler(..) => RenderComponentDeltaFlag::ContentRef,
      BindableGPUChange::Content => RenderComponentDeltaFlag::Content,
    }
  }
}

struct WebGPUTextureBackend;

impl GPUTextureBackend for WebGPUTextureBackend {
  type GPUTexture2D = GPU2DTextureView;
  type GPUSampler = GPUSamplerView;
  type GPUTexture2DBindingArray<const N: usize> = BindingResourceArray<GPU2DTextureView, N>;
  type GPUSamplerBindingArray<const N: usize> = BindingResourceArray<GPUSamplerView, N>;
  type BindingCollector = BindingBuilder;

  fn bind_texture2d(collector: &mut Self::BindingCollector, texture: &Self::GPUTexture2D) {
    collector.bind(texture);
  }

  fn bind_sampler(collector: &mut Self::BindingCollector, sampler: &Self::GPUSampler) {
    collector.bind(sampler);
  }

  fn bind_texture2d_array<const N: usize>(
    collector: &mut Self::BindingCollector,
    textures: &Self::GPUTexture2DBindingArray<N>,
  ) {
    collector.bind(textures);
  }

  fn bind_sampler_array<const N: usize>(
    collector: &mut Self::BindingCollector,
    samplers: &Self::GPUSamplerBindingArray<N>,
  ) {
    collector.bind(samplers);
  }

  fn update_texture2d_array<const N: usize>(
    textures: &mut Self::GPUTexture2DBindingArray<N>,
    source: impl Iterator<Item = Self::GPUTexture2D>,
  ) {
    let source: Vec<_> = source.collect();
    *textures = BindingResourceArray::<GPU2DTextureView, N>::new(Arc::new(source));
  }

  fn update_sampler_array<const N: usize>(
    samplers: &mut Self::GPUSamplerBindingArray<N>,
    source: impl Iterator<Item = Self::GPUSampler>,
  ) {
    let source: Vec<_> = source.collect();
    *samplers = BindingResourceArray::<GPUSamplerView, N>::new(Arc::new(source));
  }
}

#[derive(Clone, Default)]
pub struct WebGPUTextureBindingSystem {
  inner: Arc<RwLock<BindlessTextureSystem<WebGPUTextureBackend>>>,
}

impl Stream for WebGPUTextureBindingSystem {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Option<Self::Item>> {
    // todo, slab compact
    let mut inner = self.inner.write().unwrap();
    inner.maintain();

    Poll::Pending
  }
}

impl ShaderPassBuilder for WebGPUTextureBindingSystem {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    let mut inner = self.inner.write().unwrap();
    inner.bind_system_self(&mut ctx.binding)
  }
}
impl ShaderHashProvider for WebGPUTextureBindingSystem {}
impl ShaderGraphProvider for WebGPUTextureBindingSystem {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let inner = self.inner.read().unwrap();
    inner.register_system_self(builder);
    Ok(())
  }
}

impl WebGPUTextureBindingSystem {
  fn register_texture(&self, t: GPU2DTextureView) -> Texture2DHandle {
    let mut inner = self.inner.write().unwrap();
    inner.register_texture(t)
  }
  fn deregister_texture(&self, t: Texture2DHandle) {
    let mut inner = self.inner.write().unwrap();
    inner.deregister_texture(t)
  }
  fn register_sampler(&self, t: GPUSamplerView) -> SamplerHandle {
    let mut inner = self.inner.write().unwrap();
    inner.register_sampler(t)
  }
  fn deregister_sampler(&self, t: SamplerHandle) {
    let mut inner = self.inner.write().unwrap();
    inner.deregister_sampler(t)
  }

  pub fn bind_texture(&self, binding: &mut BindingBuilder, handle: Texture2DHandle) {
    // todo, avoid lock access if bindless enabled
    let mut inner = self.inner.write().unwrap();
    inner.bind_texture2d(binding, handle)
  }

  pub fn bind_sampler(&self, binding: &mut BindingBuilder, handle: SamplerHandle) {
    let mut inner = self.inner.write().unwrap();
    inner.bind_sampler(binding, handle)
  }

  pub fn bind_system(&self, binding: &mut BindingBuilder) {
    let mut inner = self.inner.write().unwrap();
    inner.bind_system_self(binding)
  }

  pub fn shader_bind_sampler(
    &self,
    builder: &mut ShaderGraphBindGroupDirectBuilder,
    handle: SamplerHandle,
  ) -> Node<ShaderSampler> {
    let inner = self.inner.read().unwrap();
    inner.register_shader_sampler(builder, handle)
  }

  pub fn shader_bind_texture(
    &self,
    builder: &mut ShaderGraphBindGroupDirectBuilder,
    handle: Texture2DHandle,
  ) -> Node<ShaderTexture2D> {
    let inner = self.inner.read().unwrap();
    inner.register_shader_texture2d(builder, handle)
  }

  pub fn shader_system(&self, builder: &mut ShaderGraphRenderPipelineBuilder) {
    let inner = self.inner.read().unwrap();
    inner.register_system_self(builder)
  }

  // todo, this is current not used but provides better abstraction
  pub fn map_texture_stream(
    &self,
    input: impl Stream<Item = GPU2DTextureView>,
  ) -> impl Stream<Item = Texture2DHandle> {
    let sys = self.clone();
    let mut previous = None;
    input.map(move |texture| {
      let handle = sys.register_texture(texture);
      if let Some(previous) = previous.replace(handle) {
        sys.deregister_texture(previous);
      }
      handle
    })
  }

  // todo, this is current not used but provides better abstraction
  pub fn map_sampler_stream(
    &self,
    input: impl Stream<Item = GPUSamplerView>,
  ) -> impl Stream<Item = SamplerHandle> {
    let sys = self.clone();
    let mut previous = None;
    input.map(move |sampler| {
      let handle = sys.register_sampler(sampler);
      if let Some(previous) = previous.replace(handle) {
        sys.deregister_sampler(previous);
      }
      handle
    })
  }
}
