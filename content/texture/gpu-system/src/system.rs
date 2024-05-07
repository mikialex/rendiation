use core::{
  hash::Hash,
  pin::Pin,
  task::{Context, Poll},
};
use std::sync::{Arc, RwLock};

use futures::{stream::FusedStream, Stream};
use rendiation_webgpu::*;

use crate::*;

#[derive(Clone)]
pub struct GPUTextureBindingSystem {
  bindless_enabled: bool,
  inner: Arc<RwLock<BindlessTextureSystem>>,
}

impl GPUTextureBindingSystem {
  pub fn new(
    gpu: &GPU,
    prefer_enable_bindless: bool,
    bindless_minimal_effective_count: usize,
  ) -> Self {
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
      <= bindless_minimal_effective_count as u32 + 128
      || info.supported_limits.max_samplers_per_shader_stage
        <= bindless_minimal_effective_count as u32 + 128
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

impl Stream for GPUTextureBindingSystem {
  type Item = ();

  fn poll_next(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Option<Self::Item>> {
    // todo, slab reorder compact?
    let mut inner = self.inner.write().unwrap();
    inner.maintain();

    Poll::Pending
  }
}
impl FusedStream for GPUTextureBindingSystem {
  fn is_terminated(&self) -> bool {
    false
  }
}

impl ShaderPassBuilder for GPUTextureBindingSystem {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.bind_system(&mut ctx.binding)
  }
}
impl ShaderHashProvider for GPUTextureBindingSystem {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.bindless_enabled.hash(hasher)
  }
  shader_hash_type_id! {}
}
impl GraphicsShaderProvider for GPUTextureBindingSystem {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.shader_system(builder);
    Ok(())
  }
}

impl GPUTextureBindingSystem {
  pub fn register_texture(&self, t: GPU2DTextureView) -> Texture2DHandle {
    let mut inner = self.inner.write().unwrap();
    inner.register_texture(t)
  }
  pub fn deregister_texture(&self, t: Texture2DHandle) {
    let mut inner = self.inner.write().unwrap();
    inner.deregister_texture(t)
  }
  pub fn register_sampler(&self, t: GPUSamplerView) -> SamplerHandle {
    let mut inner = self.inner.write().unwrap();
    inner.register_sampler(t)
  }
  pub fn deregister_sampler(&self, t: SamplerHandle) {
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
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: SamplerHandle,
  ) -> HandleNode<ShaderSampler> {
    let inner = self.inner.read().unwrap();
    inner.register_shader_sampler(builder, handle)
  }

  pub fn shader_bind_texture(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: Texture2DHandle,
  ) -> HandleNode<ShaderTexture2D> {
    let inner = self.inner.read().unwrap();
    inner.register_shader_texture2d(builder, handle)
  }

  pub fn shader_system(&self, builder: &mut ShaderRenderPipelineBuilder) {
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
  pub fn maybe_sample_texture2d_indirect_and_bind_shader(
    &self,
    binding: &mut ShaderBindGroupDirectBuilder,
    reg: &SemanticRegistry,
    host_handles: (Texture2DHandle, SamplerHandle),
    device_handles: (Node<Texture2DHandle>, Node<SamplerHandle>),
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    if self.bindless_enabled {
      let textures = reg
        .query_typed_both_stage::<BindlessTexturesInShader>()
        .unwrap();

      let samplers = reg
        .query_typed_both_stage::<BindlessSamplersInShader>()
        .unwrap();

      let texture = textures.index(device_handles.0);
      let sampler = samplers.index(device_handles.1);
      // todo currently mipmap is not supported
      texture.sample_level(sampler, uv, val(0.))
    } else {
      let texture = self.shader_bind_texture(binding, host_handles.0);
      let sampler = self.shader_bind_sampler(binding, host_handles.1);
      texture.sample(sampler, uv)
    }
  }
}
