// we design this crate to provide an abstraction over different global gpu texture management
// strategy with graphics api agnostic in mind

// we could not depend on rendiation_shader_api theoretically if we abstract over shader node
// compose but that will too complicated
use rendiation_shader_api::*;
use slab::Slab;
pub type Texture2DHandle = u32;
pub type SamplerHandle = u32;

// todo, support runtime size by query client limitation
pub const MAX_TEXTURE_BINDING_ARRAY_LENGTH: usize = 8192;
pub const MAX_SAMPLER_BINDING_ARRAY_LENGTH: usize = 8192;

pub trait GPUTextureBackend {
  type GPUTexture2D: ShaderBindingProvider<Node = ShaderHandlePtr<ShaderTexture2D>> + Clone;
  type GPUSampler: ShaderBindingProvider<Node = ShaderHandlePtr<ShaderSampler>> + Clone;
  type GPUTexture2DBindingArray<const N: usize>: ShaderBindingProvider<Node = ShaderHandlePtr<BindingArray<ShaderHandlePtr<ShaderTexture2D>, N>>>
    + Default;
  type GPUSamplerBindingArray<const N: usize>: ShaderBindingProvider<Node = ShaderHandlePtr<BindingArray<ShaderHandlePtr<ShaderSampler>, N>>>
    + Default;

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

  fn register_shader_texture2d(
    builder: &mut ShaderBindGroupDirectBuilder,
    texture: &Self::GPUTexture2D,
  ) -> HandleNode<ShaderTexture2D>;
  fn register_shader_sampler(
    builder: &mut ShaderBindGroupDirectBuilder,
    sampler: &Self::GPUSampler,
  ) -> HandleNode<ShaderSampler>;
  fn register_shader_texture2d_array(
    builder: &mut ShaderRenderPipelineBuilder,
    textures: &Self::GPUTexture2DBindingArray<MAX_TEXTURE_BINDING_ARRAY_LENGTH>,
  ) -> BindingPreparer<
    ShaderHandlePtr<
      BindingArray<ShaderHandlePtr<ShaderTexture2D>, MAX_TEXTURE_BINDING_ARRAY_LENGTH>,
    >,
  >;
  fn register_shader_sampler_array(
    builder: &mut ShaderRenderPipelineBuilder,
    samplers: &Self::GPUSamplerBindingArray<MAX_SAMPLER_BINDING_ARRAY_LENGTH>,
  ) -> BindingPreparer<
    ShaderHandlePtr<BindingArray<ShaderHandlePtr<ShaderSampler>, MAX_SAMPLER_BINDING_ARRAY_LENGTH>>,
  >;

  /// note, we should design some interface to partial update the array
  /// but the wgpu not support partial update at all, so we not bother to do this now.
  ///
  /// the Option None case is to match the hole in linear allocated array, the implementation could
  /// fill this by default value or use other proper ways to handle this case
  fn update_texture2d_array<const N: usize>(
    textures: &mut Self::GPUTexture2DBindingArray<N>,
    source: Vec<Option<Self::GPUTexture2D>>,
  );

  fn update_sampler_array<const N: usize>(
    samplers: &mut Self::GPUSamplerBindingArray<N>,
    source: Vec<Option<Self::GPUSampler>>,
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
  fn maintain(&mut self);
}

pub trait AbstractTraditionalTextureSystem<B: GPUTextureBackend> {
  fn bind_texture2d(&mut self, collector: &mut B::BindingCollector, handle: Texture2DHandle);
  fn bind_sampler(&mut self, collector: &mut B::BindingCollector, handle: SamplerHandle);

  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: Texture2DHandle,
  ) -> HandleNode<ShaderTexture2D>;
  fn register_shader_sampler(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: SamplerHandle,
  ) -> HandleNode<ShaderSampler>;

  // note, we do not need to provide abstraction over Node<texture> direct sample
}

pub trait AbstractIndirectGPUTextureSystem<B: GPUTextureBackend> {
  fn bind_system_self(&mut self, collector: &mut B::BindingCollector);
  fn register_system_self(&self, builder: &mut ShaderRenderPipelineBuilder);
  fn sample_texture2d_indirect(
    &self,
    reg: &SemanticRegistry,
    shader_texture_handle: Node<Texture2DHandle>,
    shader_sampler_handle: Node<SamplerHandle>,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>>;
}

pub struct TraditionalPerDrawBindingSystem<B: GPUTextureBackend> {
  textures: Slab<B::GPUTexture2D>,
  samplers: Slab<B::GPUSampler>,
}

impl<B: GPUTextureBackend> Default for TraditionalPerDrawBindingSystem<B> {
  fn default() -> Self {
    Self {
      textures: Default::default(),
      samplers: Default::default(),
    }
  }
}

impl<B: GPUTextureBackend> AbstractGPUTextureSystemBase<B> for TraditionalPerDrawBindingSystem<B> {
  fn register_texture(&mut self, t: B::GPUTexture2D) -> Texture2DHandle {
    self.textures.insert(t) as u32
  }
  fn deregister_texture(&mut self, t: Texture2DHandle) {
    self.textures.remove(t as usize);
  }
  fn register_sampler(&mut self, t: B::GPUSampler) -> SamplerHandle {
    self.samplers.insert(t) as u32
  }
  fn deregister_sampler(&mut self, t: SamplerHandle) {
    self.samplers.remove(t as usize);
  }
  fn maintain(&mut self) {}
}

impl<B: GPUTextureBackend> AbstractTraditionalTextureSystem<B>
  for TraditionalPerDrawBindingSystem<B>
{
  fn bind_texture2d(&mut self, collector: &mut B::BindingCollector, handle: Texture2DHandle) {
    let texture = self.textures.get(handle as usize).unwrap();
    B::bind_texture2d(collector, texture);
  }

  fn bind_sampler(&mut self, collector: &mut B::BindingCollector, handle: SamplerHandle) {
    let sampler = self.samplers.get(handle as usize).unwrap();
    B::bind_sampler(collector, sampler);
  }

  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: Texture2DHandle,
  ) -> HandleNode<ShaderTexture2D> {
    let texture = self.textures.get(handle as usize).unwrap();
    B::register_shader_texture2d(builder, texture)
  }

  fn register_shader_sampler(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: SamplerHandle,
  ) -> HandleNode<ShaderSampler> {
    let sampler = self.samplers.get(handle as usize).unwrap();
    B::register_shader_sampler(builder, sampler)
  }
}

pub struct BindlessTextureSystem<B: GPUTextureBackend> {
  inner: TraditionalPerDrawBindingSystem<B>,
  texture_binding_array: B::GPUTexture2DBindingArray<MAX_TEXTURE_BINDING_ARRAY_LENGTH>,
  sampler_binding_array: B::GPUSamplerBindingArray<MAX_SAMPLER_BINDING_ARRAY_LENGTH>,
  any_changed: bool, // should we add change mark to per type?
  enable_bindless: bool,
}
impl<B: GPUTextureBackend> BindlessTextureSystem<B> {
  pub fn new(enable_bindless: bool) -> Self {
    Self {
      inner: Default::default(),
      texture_binding_array: Default::default(),
      sampler_binding_array: Default::default(),
      any_changed: true,
      enable_bindless,
    }
  }
}

/// pass through inner implementation
impl<B: GPUTextureBackend> AbstractGPUTextureSystemBase<B> for BindlessTextureSystem<B> {
  fn register_texture(&mut self, t: B::GPUTexture2D) -> Texture2DHandle {
    self.any_changed = true;
    self.inner.register_texture(t)
  }
  fn deregister_texture(&mut self, t: Texture2DHandle) {
    self.any_changed = true;
    self.inner.deregister_texture(t)
  }
  fn register_sampler(&mut self, t: B::GPUSampler) -> SamplerHandle {
    self.any_changed = true;
    self.inner.register_sampler(t)
  }
  fn deregister_sampler(&mut self, t: SamplerHandle) {
    self.any_changed = true;
    self.inner.deregister_sampler(t)
  }
  fn maintain(&mut self) {
    if !self.any_changed {
      return;
    }
    self.any_changed = false;
    self.inner.maintain();

    if !self.enable_bindless {
      return;
    }

    // this is not good, maybe we should impl slab by ourself?
    fn slab_to_hole_vec<T: Clone>(s: &Slab<T>) -> Vec<Option<T>> {
      let mut r = Vec::with_capacity(s.capacity());
      s.iter().for_each(|(idx, v)| {
        while idx >= r.len() {
          r.push(None)
        }
        r[idx] = v.clone().into();
      });
      r
    }

    B::update_sampler_array(
      &mut self.sampler_binding_array,
      slab_to_hole_vec(&self.inner.samplers),
    );
    B::update_texture2d_array(
      &mut self.texture_binding_array,
      slab_to_hole_vec(&self.inner.textures),
    );
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
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: Texture2DHandle,
  ) -> HandleNode<ShaderTexture2D> {
    self.inner.register_shader_texture2d(builder, handle)
  }

  fn register_shader_sampler(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: SamplerHandle,
  ) -> HandleNode<ShaderSampler> {
    self.inner.register_shader_sampler(builder, handle)
  }
}
both!(
  BindlessTexturesInShader,
  ShaderHandlePtr<BindingArray<ShaderHandlePtr<ShaderTexture2D>, MAX_TEXTURE_BINDING_ARRAY_LENGTH>>
);
both!(
  BindlessSamplersInShader,
  ShaderHandlePtr<BindingArray<ShaderHandlePtr<ShaderSampler>, MAX_SAMPLER_BINDING_ARRAY_LENGTH>>
);

impl<B: GPUTextureBackend> AbstractIndirectGPUTextureSystem<B> for BindlessTextureSystem<B> {
  fn bind_system_self(&mut self, collector: &mut B::BindingCollector) {
    B::bind_texture2d_array(collector, &self.texture_binding_array);
    B::bind_sampler_array(collector, &self.sampler_binding_array);
  }

  fn register_system_self(&self, builder: &mut ShaderRenderPipelineBuilder) {
    B::register_shader_texture2d_array(builder, &self.texture_binding_array).using_graphics_pair(
      builder,
      |r, textures| {
        r.register_typed_both_stage::<BindlessTexturesInShader>(textures);
      },
    );
    B::register_shader_sampler_array(builder, &self.sampler_binding_array).using_graphics_pair(
      builder,
      |r, samplers| {
        r.register_typed_both_stage::<BindlessSamplersInShader>(samplers);
      },
    );
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
