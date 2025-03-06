use crate::*;

// todo, improve performance using value refed query
pub struct TraditionalPerDrawBindingSystemSource {
  pub default_tex: GPU2DTextureView,
  pub default_sampler: GPUSamplerView,
  pub textures: BoxedDynReactiveQuery<Texture2DHandle, GPU2DTextureView>,
  pub samplers: BoxedDynReactiveQuery<SamplerHandle, GPUSamplerView>,
}

impl ReactiveGeneralQuery for TraditionalPerDrawBindingSystemSource {
  type Output = Box<dyn DynAbstractGPUTextureSystem>;

  fn poll_query(&mut self, cx: &mut Context) -> Self::Output {
    let (_, textures) = self.textures.poll_changes(cx);
    let (_, samplers) = self.samplers.poll_changes(cx);
    Box::new(TraditionalPerDrawBindingSystem {
      textures,
      samplers,
      default_tex: self.default_tex.clone(),
      default_sampler: self.default_sampler.clone(),
    })
  }
}

#[derive(Clone)]
pub struct TraditionalPerDrawBindingSystem {
  pub default_tex: GPU2DTextureView,
  pub default_sampler: GPUSamplerView,
  pub textures: BoxedDynQuery<Texture2DHandle, GPU2DTextureView>,
  pub samplers: BoxedDynQuery<SamplerHandle, GPUSamplerView>,
}

impl AbstractGPUTextureSystem for TraditionalPerDrawBindingSystem {
  type RegisteredShaderTexture = BindingNode<ShaderTexture2D>;
  type RegisteredShaderSampler = BindingNode<ShaderSampler>;

  fn bind_texture2d(&self, collector: &mut BindingBuilder, handle: Texture2DHandle) {
    let texture = if handle == u32::MAX {
      self.default_tex.clone()
    } else {
      self.textures.access(&handle).unwrap()
    };
    collector.bind(&texture);
  }
  fn bind_sampler(&self, collector: &mut BindingBuilder, handle: SamplerHandle) {
    let sampler = if handle == u32::MAX {
      self.default_sampler.clone()
    } else {
      self.samplers.access(&handle).unwrap()
    };
    collector.bind(&sampler);
  }
  fn bind_system_self(&self, _: &mut BindingBuilder) {}

  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderBindGroupBuilder,
    handle: Texture2DHandle,
    _: Node<Texture2DHandle>,
  ) -> BindingNode<ShaderTexture2D> {
    let texture = if handle == u32::MAX {
      self.default_tex.clone()
    } else {
      self.textures.access(&handle).unwrap()
    };
    builder.bind_by(&texture)
  }

  fn register_shader_sampler(
    &self,
    builder: &mut ShaderBindGroupBuilder,
    handle: SamplerHandle,
    _: Node<Texture2DHandle>,
  ) -> BindingNode<ShaderSampler> {
    let sampler = if handle == u32::MAX {
      self.default_sampler.clone()
    } else {
      self.samplers.access(&handle).unwrap()
    };
    builder.bind_by(&sampler)
  }

  fn register_system_self(&self, _: &mut ShaderRenderPipelineBuilder) {}
  fn register_system_self_for_compute(&self, _: &mut ShaderBindGroupBuilder) {}

  fn sample_texture2d(
    &self,
    _: &SemanticRegistry,
    shader_texture_handle: Self::RegisteredShaderTexture,
    shader_sampler_handle: Self::RegisteredShaderSampler,
    uv: Node<Vec2<f32>>,
  ) -> Node<Vec4<f32>> {
    shader_texture_handle.sample(shader_sampler_handle, uv)
  }

  fn as_indirect_system(&self) -> Option<&dyn AbstractIndirectGPUTextureSystem> {
    None
  }
}
