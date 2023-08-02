use crate::*;

mod cube;
mod d2;
mod pair;
mod sampler;

pub use cube::*;
pub use d2::*;
pub use pair::*;
pub use sampler::*;

/// note: we could design beautiful apis like Stream<Item = GPU2DTextureView> -> Stream<Item =
/// Texture2DHandle>, but for now, we require bindless downgrade ability, so we directly combined
/// the handle with the resource in BindableGPUChange
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
    source: Vec<Option<Self::GPUTexture2D>>,
  ) {
    let first = source[0].clone().unwrap(); // we make sure the first is the default
    let source: Vec<_> = source
      .into_iter()
      .map(|v| v.unwrap_or(first.clone()))
      .collect();
    *textures = BindingResourceArray::<GPU2DTextureView, N>::new(Arc::new(source));
  }

  fn update_sampler_array<const N: usize>(
    samplers: &mut Self::GPUSamplerBindingArray<N>,
    source: Vec<Option<Self::GPUSampler>>,
  ) {
    let first = source[0].clone().unwrap(); // we make sure the first is the default
    let source: Vec<_> = source
      .into_iter()
      .map(|v| v.unwrap_or(first.clone()))
      .collect();
    *samplers = BindingResourceArray::<GPUSamplerView, N>::new(Arc::new(source));
  }
}

#[derive(Clone)]
pub struct WebGPUTextureBindingSystem {
  bindless_enabled: bool,
  inner: Arc<RwLock<BindlessTextureSystem<WebGPUTextureBackend>>>,
}

impl WebGPUTextureBindingSystem {
  pub fn new(gpu: &GPU, prefer_enable_bindless: bool) -> Self {
    let info = gpu.info();
    let mut bindless_effectively_supported = info
      .supported_features
      .contains(Features::TEXTURE_BINDING_ARRAY)
      && info
        .supported_features
        .contains(Features::PARTIALLY_BOUND_BINDING_ARRAY)
      && info
        .supported_features
        .contains(Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING);

    // we estimate that the texture used except under the binding system will not exceed 128 per
    // shader stage
    if info.supported_limits.max_sampled_textures_per_shader_stage
      < MAX_TEXTURE_BINDING_ARRAY_LENGTH as u32 + 128
      || info.supported_limits.max_samplers_per_shader_stage
        < MAX_SAMPLER_BINDING_ARRAY_LENGTH as u32 + 128
    {
      bindless_effectively_supported = false;
    }

    let bindless_enabled = prefer_enable_bindless && bindless_effectively_supported;

    Self {
      bindless_enabled,
      inner: Arc::new(RwLock::new(BindlessTextureSystem::new(bindless_enabled))),
    }
  }
}

impl Stream for WebGPUTextureBindingSystem {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Option<Self::Item>> {
    // todo, slab reorder compact?
    let mut inner = self.inner.write().unwrap();
    inner.maintain();

    Poll::Pending
  }
}

impl ShaderPassBuilder for WebGPUTextureBindingSystem {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.bind_system(&mut ctx.binding)
  }
}
impl ShaderHashProvider for WebGPUTextureBindingSystem {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.bindless_enabled.hash(hasher)
  }
}
impl ShaderGraphProvider for WebGPUTextureBindingSystem {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    self.shader_system(builder);
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
    if self.bindless_enabled {
      return;
    }
    // indeed, we lost performance in none bindless path by this lock access. This definitely has
    // improvement space
    let mut inner = self.inner.write().unwrap();
    inner.bind_texture2d(binding, handle)
  }

  pub fn bind_sampler(&self, binding: &mut BindingBuilder, handle: SamplerHandle) {
    if self.bindless_enabled {
      return;
    }
    // ditto
    let mut inner = self.inner.write().unwrap();
    inner.bind_sampler(binding, handle)
  }

  pub fn bind_system(&self, binding: &mut BindingBuilder) {
    if !self.bindless_enabled {
      return;
    }
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
    if !self.bindless_enabled {
      return;
    }
    let inner = self.inner.read().unwrap();
    inner.register_system_self(builder)
  }

  // note, when we unify the bind and bindless case, one bad point is the traditional bind
  // path requires shader binding register, so if we blindly unify the sample method, each sample
  // call will result distinct new binding point registered in traditional bind case, and that's
  // bad. so we name our method to explicitly say we maybe do a bind register on shader when
  // bindless is disabled.
  //
  // Even if the bindless is enabled, the user could mixed the usage with the binding and bindless
  // freely. We could expose the underlayer indirect method for user side to solve the reuse and
  // downgrade in future.
  #[allow(clippy::too_many_arguments)]
  pub fn maybe_sample_texture2d_indirect_and_bind_shader(
    &self,
    binding: &mut ShaderGraphBindGroupDirectBuilder,
    reg: &SemanticRegistry,
    texture_handle: Texture2DHandle,
    shader_texture_handle: Node<Texture2DHandle>,
    sample_handle: SamplerHandle,
    shader_sampler_handle: Node<SamplerHandle>,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    if self.bindless_enabled {
      let textures = reg
        .query_typed_both_stage::<BindlessTexturesInShader>()
        .unwrap();

      let samplers = reg
        .query_typed_both_stage::<BindlessSamplersInShader>()
        .unwrap();

      let texture = textures.index(shader_texture_handle);
      let sampler = samplers.index(shader_sampler_handle);
      texture.sample(sampler, uv)
    } else {
      let texture = self.shader_bind_texture(binding, texture_handle);
      let sampler = self.shader_bind_sampler(binding, sample_handle);
      texture.sample(sampler, uv)
    }
  }
}
