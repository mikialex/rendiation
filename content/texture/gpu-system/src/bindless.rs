use crate::*;

pub fn is_bindless_supported_on_this_gpu(info: &GPUInfo, max_binding_length: u32) -> bool {
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
  if info.supported_limits.max_sampled_textures_per_shader_stage <= max_binding_length + 128
    || info.supported_limits.max_samplers_per_shader_stage <= max_binding_length + 128
  {
    bindless_effectively_supported = false;
  }
  bindless_effectively_supported
}

pub struct BindlessTextureSystemSource {
  texture2d: BindingArrayMaintainer<u32, GPU2DTextureView>,
  sampler: BindingArrayMaintainer<u32, GPUSamplerView>,
}

impl BindlessTextureSystemSource {
  pub fn new(
    texture2d: impl ReactiveCollection<u32, GPU2DTextureView>,
    default_2d: GPU2DTextureView,
    sampler: impl ReactiveCollection<u32, GPUSamplerView>,
    default_sampler: GPUSamplerView,
    max_binding_length: u32,
  ) -> Self {
    Self {
      texture2d: BindingArrayMaintainer::new(
        texture2d.into_boxed(),
        default_2d,
        max_binding_length,
      ),
      sampler: BindingArrayMaintainer::new(
        sampler.into_boxed(),
        default_sampler,
        max_binding_length,
      ),
    }
  }
}

both!(
  BindlessTexturesInShader,
  ShaderHandlePtr<BindingArray<ShaderHandlePtr<ShaderTexture2D>>>
);
both!(
  BindlessSamplersInShader,
  ShaderHandlePtr<BindingArray<ShaderHandlePtr<ShaderSampler>>>
);

impl ReactiveState for BindlessTextureSystemSource {
  type State = BindlessTextureSystem;

  fn poll_current(&mut self, cx: &mut Context) -> Self::State {
    BindlessTextureSystem {
      texture_binding_array: self.texture2d.poll_update(cx),
      sampler_binding_array: self.sampler.poll_update(cx),
    }
  }
}

pub struct BindlessTextureSystem {
  texture_binding_array: BindingResourceArray<GPU2DTextureView>,
  sampler_binding_array: BindingResourceArray<GPUSamplerView>,
}

impl AbstractIndirectGPUTextureSystem for BindlessTextureSystem {
  fn bind_system_self(&self, collector: &mut BindingBuilder) {
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
