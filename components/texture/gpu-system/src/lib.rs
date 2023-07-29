// we could not depend on shadergraph theoretically if we abstract over shader node compose
// but that will too complicated
use shadergraph::*;
pub type Texture2DHandle = u32;
pub type SamplerHandle = u32;

pub trait GPUTextureBackend {
  type GPUTexture2D: ShaderBindingProvider<Node = ShaderTexture2D>;
  type GPUSampler: ShaderBindingProvider<Node = ShaderSampler>;
  type GPUTexture2DBindingArray<const N: usize>: ShaderBindingProvider<
    Node = BindingArray<ShaderTexture2D, N>,
  >;
  type GPUSamplerBindingArray<const N: usize>: ShaderBindingProvider<
    Node = BindingArray<ShaderSampler, N>,
  >;

  type BindingCollector;
  fn bind_texture2d(collector: &mut Self::BindingCollector, texture: &Self::GPUTexture2D);
  fn bind_sampler(collector: &mut Self::BindingCollector, sampler: &Self::GPUSampler);
  fn bind_texture2d_array<const N: usize>(
    collector: &mut Self::BindingCollector,
    textures: &Self::GPUTexture2DBindingArray<N>,
  );
  fn bind_sampler_array<const N: usize>(
    collector: &mut Self::BindingCollector,
    samplers: &Self::GPUSamplerBindingArray<N>,
  );
}

pub trait GPUTextureAdvanceBackend: GPUTextureBackend {
  type GPUStorageBuffer<T>: ShaderBindingProvider<Node = T>;

  fn bind_storage<T>(collector: &mut Self::BindingCollector, buffer: &Self::GPUStorageBuffer<T>);
}

pub trait AbstractGPUTextureSystemBase<B: GPUTextureBackend> {
  fn register_texture(&mut self, t: B::GPUTexture2D) -> Texture2DHandle;
  fn deregister_texture(&mut self, t: Texture2DHandle);
  fn register_sampler(&mut self, t: B::GPUSampler) -> SamplerHandle;
  fn deregister_sampler(&mut self, t: SamplerHandle);
}

pub trait AbstractTraditionalTextureSystem<B: GPUTextureBackend> {
  fn bind_texture2d(&mut self, collector: &mut B::BindingCollector, handle: Texture2DHandle);
  fn bind_sampler(&mut self, collector: &mut B::BindingCollector, handle: SamplerHandle);

  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderGraphBindGroupDirectBuilder,
    handle: Texture2DHandle,
  ) -> Node<ShaderTexture2D>;
  fn register_shader_sampler(
    &self,
    builder: &mut ShaderGraphBindGroupDirectBuilder,
    handle: SamplerHandle,
  ) -> Node<ShaderSampler>;

  // note, we do not need to provide abstraction over Node<texture> direct sample
}

pub trait AbstractIndirectGPUTextureSystem<B: GPUTextureBackend> {
  fn bind_system_self(&mut self, collector: &mut B::BindingCollector);
  fn register_system_self(&self, builder: &mut ShaderGraphRenderPipelineBuilder);
  fn sample_texture2d_indirect(
    &self,
    reg: &SemanticRegistry,
    shader_texture_handle: Node<Texture2DHandle>,
    shader_sampler_handle: Node<SamplerHandle>,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>>;
}

pub trait SlabAllocator<T> {
  fn allocate(&mut self, item: T) -> u32;
  fn deallocate(&mut self, item: u32);
  fn get(&self, handle: u32) -> &T;
}

pub struct TraditionalPerDrawBindingSystem<B: GPUTextureBackend> {
  textures: Box<dyn SlabAllocator<B::GPUTexture2D>>,
  samplers: Box<dyn SlabAllocator<B::GPUSampler>>,
}

impl<B: GPUTextureBackend> AbstractGPUTextureSystemBase<B> for TraditionalPerDrawBindingSystem<B> {
  fn register_texture(&mut self, t: B::GPUTexture2D) -> Texture2DHandle {
    self.textures.allocate(t)
  }
  fn deregister_texture(&mut self, t: Texture2DHandle) {
    self.textures.deallocate(t)
  }
  fn register_sampler(&mut self, t: B::GPUSampler) -> SamplerHandle {
    self.samplers.allocate(t)
  }
  fn deregister_sampler(&mut self, t: SamplerHandle) {
    self.samplers.deallocate(t)
  }
}

impl<B: GPUTextureBackend> AbstractTraditionalTextureSystem<B>
  for TraditionalPerDrawBindingSystem<B>
{
  fn bind_texture2d(&mut self, collector: &mut B::BindingCollector, handle: Texture2DHandle) {
    let texture = self.textures.get(handle);
    B::bind_texture2d(collector, texture);
  }

  fn bind_sampler(&mut self, collector: &mut B::BindingCollector, handle: SamplerHandle) {
    let sampler = self.samplers.get(handle);
    B::bind_sampler(collector, sampler);
  }

  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderGraphBindGroupDirectBuilder,
    handle: Texture2DHandle,
  ) -> Node<ShaderTexture2D> {
    let texture = self.textures.get(handle);
    builder.uniform_by(texture)
  }

  fn register_shader_sampler(
    &self,
    builder: &mut ShaderGraphBindGroupDirectBuilder,
    handle: SamplerHandle,
  ) -> Node<ShaderSampler> {
    let sampler = self.samplers.get(handle);
    builder.uniform_by(sampler)
  }
}

pub struct BindlessTextureSystem<B: GPUTextureBackend> {
  inner: TraditionalPerDrawBindingSystem<B>,
  texture_binding_array: B::GPUTexture2DBindingArray<1024>,
  sampler_binding_array: B::GPUSamplerBindingArray<1024>,
}

/// pass through inner implementation
impl<B: GPUTextureBackend> AbstractGPUTextureSystemBase<B> for BindlessTextureSystem<B> {
  fn register_texture(&mut self, t: B::GPUTexture2D) -> Texture2DHandle {
    self.inner.register_texture(t)
  }
  fn deregister_texture(&mut self, t: Texture2DHandle) {
    self.inner.deregister_texture(t)
  }
  fn register_sampler(&mut self, t: B::GPUSampler) -> SamplerHandle {
    self.inner.register_sampler(t)
  }
  fn deregister_sampler(&mut self, t: SamplerHandle) {
    self.inner.deregister_sampler(t)
  }
}

impl<B: GPUTextureBackend> AbstractTraditionalTextureSystem<B> for BindlessTextureSystem<B> {
  fn bind_texture2d(&mut self, collector: &mut B::BindingCollector, handle: Texture2DHandle) {
    self.inner.bind_texture2d(collector, handle)
  }

  fn bind_sampler(&mut self, collector: &mut B::BindingCollector, handle: SamplerHandle) {
    self.inner.bind_sampler(collector, handle)
  }

  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderGraphBindGroupDirectBuilder,
    handle: Texture2DHandle,
  ) -> Node<ShaderTexture2D> {
    self.inner.register_shader_texture2d(builder, handle)
  }

  fn register_shader_sampler(
    &self,
    builder: &mut ShaderGraphBindGroupDirectBuilder,
    handle: SamplerHandle,
  ) -> Node<ShaderSampler> {
    self.inner.register_shader_sampler(builder, handle)
  }
}

both!(BindlessTexturesInShader, BindingArray<ShaderTexture2D, 1024>);
both!(BindlessSamplersInShader, BindingArray<ShaderSampler, 1024>);

impl<B: GPUTextureBackend> AbstractIndirectGPUTextureSystem<B> for BindlessTextureSystem<B> {
  fn bind_system_self(&mut self, collector: &mut B::BindingCollector) {
    B::bind_texture2d_array(collector, &self.texture_binding_array);
    B::bind_sampler_array(collector, &self.sampler_binding_array);
  }

  fn register_system_self(&self, builder: &mut ShaderGraphRenderPipelineBuilder) {
    builder
      .uniform_by(&self.texture_binding_array)
      .using_both(builder, |r, textures| {
        r.register_typed_both_stage::<BindlessTexturesInShader>(textures);
      });
    builder
      .uniform_by(&self.sampler_binding_array)
      .using_both(builder, |r, samplers| {
        r.register_typed_both_stage::<BindlessSamplersInShader>(samplers);
      });
  }

  fn sample_texture2d_indirect(
    &self,
    reg: &SemanticRegistry,
    shader_texture_handle: Node<Texture2DHandle>,
    shader_sampler_handle: Node<SamplerHandle>,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    let textures = reg
      .query_typed_both_stage::<BindlessTexturesInShader>()
      .unwrap();

    let samplers = reg
      .query_typed_both_stage::<BindlessSamplersInShader>()
      .unwrap();

    let texture = textures.index(shader_texture_handle);
    let sampler = samplers.index(shader_sampler_handle);
    texture.sample(sampler, uv)
  }
}
