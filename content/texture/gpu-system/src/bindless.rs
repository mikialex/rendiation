use std::{
  array,
  marker::PhantomData,
  sync::Arc,
  task::{Context, Poll},
};

use crate::*;

pub fn is_bindless_supported_on_this_gpu(gpu: &GPU) -> bool {
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
    <= MAX_BINDING_ARRAY_LENGTH as u32 + 128
    || info.supported_limits.max_samplers_per_shader_stage <= MAX_BINDING_ARRAY_LENGTH as u32 + 128
  {
    bindless_effectively_supported = false;
  }
  bindless_effectively_supported
}

// todo, support runtime size by query client limitation
pub const MAX_BINDING_ARRAY_LENGTH: usize = 8192;

pub struct BindingArrayMaintainer<K, V> {
  upstream: Box<dyn ReactiveCollection<K, V>>,
  array: Option<BindingResourceArray<V, MAX_BINDING_ARRAY_LENGTH>>,
  default_instance: V,
}

impl<K, V> BindingArrayMaintainer<K, V> {
  pub fn new(upstream: Box<dyn ReactiveCollection<K, V>>, default: V) -> Self {
    Self {
      upstream,
      array: Default::default(),
      default_instance: default,
    }
  }
}

impl<K, V> BindingArrayMaintainer<K, V>
where
  K: CKey + LinearIdentified,
  V: CValue,
{
  pub fn poll_update(&mut self, cx: &mut Context) {
    // detail change info is useless here because the binding array update can not be preformed
    // incrementally
    if self.upstream.poll_changes(cx).is_ready() {
      let full_view = self.upstream.access();
      let mut new_source = vec![self.default_instance; MAX_BINDING_ARRAY_LENGTH];
      for (k, v) in full_view.iter_key_value() {
        new_source[k.alloc_index() as usize] = v.clone();
      }
      self.array =
        BindingResourceArray::<V, MAX_BINDING_ARRAY_LENGTH>::new(Arc::new(new_source)).into();
    }
  }
}

pub struct BindlessTextureSystem {
  texture2d: BindingArrayMaintainer<u32, GPU2DTextureView>,
  sampler: BindingArrayMaintainer<u32, GPUSamplerView>,
}

impl BindlessTextureSystem {
  pub fn new(
    texture2d: impl ReactiveCollection<u32, GPU2DTextureView>,
    default_2d: GPU2DTextureView,
    sampler: impl ReactiveCollection<u32, GPUSamplerView>,
    default_sampler: GPUSamplerView,
  ) -> Self {
    Self {
      texture2d: BindingArrayMaintainer::new(texture2d.into_boxed(), default_2d),
      sampler: BindingArrayMaintainer::new(sampler.into_boxed(), default_sampler),
    }
  }
}

both!(
  BindlessTexturesInShader,
  ShaderHandlePtr<BindingArray<ShaderHandlePtr<ShaderTexture2D>, MAX_BINDING_ARRAY_LENGTH>>
);
both!(
  BindlessSamplersInShader,
  ShaderHandlePtr<BindingArray<ShaderHandlePtr<ShaderSampler>, MAX_BINDING_ARRAY_LENGTH>>
);

impl AbstractIndirectGPUTextureSystem for BindlessTextureSystem {
  fn bind_system_self(&mut self, collector: &mut BindingBuilder) {
    collector.bind(&self.texture_binding_array);
    collector.bind(&self.sampler_binding_array);
  }

  fn register_system_self(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder
      .bind_by(&self.texture_binding_array)
      .using_graphics_pair(builder, |r, textures| {
        r.register_typed_both_stage::<BindlessTexturesInShader>(textures);
      });
    builder
      .bind_by(&self.sampler_binding_array)
      .using_graphics_pair(builder, |r, samplers| {
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
