use crate::*;

pub fn is_bindless_supported_on_this_gpu(info: &GPUInfo, max_binding_length: u32) -> bool {
  let required_features = Features::TEXTURE_BINDING_ARRAY
    | Features::PARTIALLY_BOUND_BINDING_ARRAY
    | Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING;

  let mut bindless_effectively_supported = info.supported_features.contains(required_features);

  // we estimate that the texture used in shader that outside of bindless system will not exceed 128 per
  // shader stage
  if info.supported_limits.max_sampled_textures_per_shader_stage <= max_binding_length + 128
    || info.supported_limits.max_samplers_per_shader_stage <= max_binding_length + 128
  {
    bindless_effectively_supported = false;
  }
  bindless_effectively_supported
}

both!(
  BindlessTexturesInShader,
  ShaderBinding<BindingArray<ShaderBinding<ShaderTexture2D>>>
);
both!(
  BindlessSamplersInShader,
  ShaderBinding<BindingArray<ShaderBinding<ShaderSampler>>>
);

#[derive(Clone)]
pub struct BindlessTextureSystem {
  pub texture_binding_array: BindingResourceArray<GPU2DTextureView>,
  pub sampler_binding_array: BindingResourceArray<GPUSamplerView>,
}

impl AbstractIndirectGPUTextureSystem for BindlessTextureSystem {
  fn bind_system_self(&self, collector: &mut BindingBuilder) {
    collector.bind(&self.texture_binding_array);
    collector.bind(&self.sampler_binding_array);
  }

  fn register_system_self(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder
      .bind_by_and_prepare(&self.texture_binding_array)
      .using_graphics_pair(|r, textures| {
        r.register_typed_both_stage::<BindlessTexturesInShader>(*textures);
      });
    builder
      .bind_by_and_prepare(&self.sampler_binding_array)
      .using_graphics_pair(|r, samplers| {
        r.register_typed_both_stage::<BindlessSamplersInShader>(*samplers);
      });
  }
  fn register_system_self_for_compute(
    &self,
    builder: &mut ShaderBindGroupBuilder,
    reg: &mut SemanticRegistry,
  ) {
    let textures = builder.bind_by(&self.texture_binding_array);
    reg.register_typed_both_stage::<BindlessTexturesInShader>(textures);
    let samplers = builder.bind_by(&self.sampler_binding_array);
    reg.register_typed_both_stage::<BindlessSamplersInShader>(samplers);
  }

  fn sample_texture2d_indirect(
    &self,
    reg: &SemanticRegistry,
    shader_texture_handle: Node<Texture2DHandle>,
    shader_sampler_handle: Node<SamplerHandle>,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    let textures = reg
      .try_query_typed_both_stage::<BindlessTexturesInShader>()
      .unwrap();

    let samplers = reg
      .try_query_typed_both_stage::<BindlessSamplersInShader>()
      .unwrap();

    let texture = textures.index(shader_texture_handle);
    let sampler = samplers.index(shader_sampler_handle);

    if get_current_stage() == Some(ShaderStage::Fragment) {
      texture.sample(sampler, uv)
    } else {
      // force disable mipmap compute(because using dpdx stuff is not supported in none fragment stage)
      texture.sample_zero_level(sampler, uv)
    }
  }
}
