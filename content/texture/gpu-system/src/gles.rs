use crate::*;

// todo, improve performance using self contained collection
pub struct TraditionalPerDrawBindingSystemSource {
  pub textures: Box<dyn DynReactiveCollection<Texture2DHandle, GPU2DTextureView>>,
  pub samplers: Box<dyn DynReactiveCollection<SamplerHandle, GPUSamplerView>>,
}

impl ReactiveQuery for TraditionalPerDrawBindingSystemSource {
  type Output = Box<dyn DynAbstractGPUTextureSystem>;

  fn poll_query(&mut self, cx: &mut Context) -> Self::Output {
    let (_, textures) = self.textures.poll_changes(cx);
    let (_, samplers) = self.samplers.poll_changes(cx);
    Box::new(TraditionalPerDrawBindingSystem { textures, samplers })
  }
}

pub struct TraditionalPerDrawBindingSystem {
  pub textures: Box<dyn DynVirtualCollection<Texture2DHandle, GPU2DTextureView>>,
  pub samplers: Box<dyn DynVirtualCollection<SamplerHandle, GPUSamplerView>>,
}

impl AbstractGPUTextureSystem for TraditionalPerDrawBindingSystem {
  type RegisteredShaderTexture = HandleNode<ShaderTexture2D>;
  type RegisteredShaderSampler = HandleNode<ShaderSampler>;

  fn bind_texture2d(&self, collector: &mut BindingBuilder, handle: Texture2DHandle) {
    let texture = self.textures.access(&handle).unwrap();
    collector.bind(&texture);
  }
  fn bind_sampler(&self, collector: &mut BindingBuilder, handle: SamplerHandle) {
    let sampler = self.samplers.access(&handle).unwrap();
    collector.bind(&sampler);
  }
  fn bind_system_self(&self, _: &mut BindingBuilder) {}

  fn register_shader_texture2d(
    &self,
    builder: &mut ShaderBindGroupBuilder,
    handle: Texture2DHandle,
    _: Node<Texture2DHandle>,
  ) -> HandleNode<ShaderTexture2D> {
    let texture = self.textures.access(&handle).unwrap();
    builder.bind_by(&texture)
  }

  fn register_shader_sampler(
    &self,
    builder: &mut ShaderBindGroupBuilder,
    handle: SamplerHandle,
    _: Node<Texture2DHandle>,
  ) -> HandleNode<ShaderSampler> {
    let sampler = self.samplers.access(&handle).unwrap();
    builder.bind_by(&sampler)
  }

  fn register_system_self(&self, _: &mut ShaderRenderPipelineBuilder) {}

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
