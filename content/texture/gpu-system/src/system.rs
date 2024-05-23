use crate::*;

pub struct GPUTextureBindingSystemSource {
  base: TraditionalPerDrawBindingSystemSource,
  bindless: Option<BindlessTextureSystemSource>,
}

pub struct GPUTextureBindingSystem {
  base: TraditionalPerDrawBindingSystem,
  bindless: Option<BindlessTextureSystem>,
}

impl GPUTextureBindingSystemSource {
  pub fn new(
    info: &GPUInfo,
    texture_2d: RxCForker<Texture2DHandle, GPU2DTextureView>,
    default_2d: GPU2DTextureView,
    sampler: RxCForker<SamplerHandle, GPUSamplerView>,
    default_sampler: GPUSamplerView,
    prefer_enable_bindless: bool,
    bindless_minimal_effective_count: u32,
  ) -> Self {
    let base = TraditionalPerDrawBindingSystemSource {
      textures: Box::new(texture_2d.clone()),
      samplers: Box::new(sampler.clone()),
    };
    let bindless_effectively_supported =
      is_bindless_supported_on_this_gpu(info, bindless_minimal_effective_count);

    let bindless_enabled = prefer_enable_bindless && bindless_effectively_supported;

    let bindless = bindless_enabled.then(|| {
      BindlessTextureSystemSource::new(
        texture_2d,
        default_2d,
        sampler,
        default_sampler,
        bindless_minimal_effective_count,
      )
    });

    Self { base, bindless }
  }
}

impl ReactiveState for GPUTextureBindingSystemSource {
  type State = GPUTextureBindingSystem;

  fn poll_current(&mut self, cx: &mut Context) -> Self::State {
    GPUTextureBindingSystem {
      base: self.base.poll_current(cx),
      bindless: self.bindless.as_mut().map(|sys| sys.poll_current(cx)),
    }
  }
}

impl ShaderPassBuilder for GPUTextureBindingSystem {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.bind_system(&mut ctx.binding)
  }
}
impl ShaderHashProvider for GPUTextureBindingSystem {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.bindless.is_some().hash(hasher)
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
  pub fn bind_texture(&self, binding: &mut BindingBuilder, handle: Texture2DHandle) {
    if self.bindless.is_some() {
      return;
    }
    self.base.bind_texture2d(binding, handle)
  }

  pub fn bind_sampler(&self, binding: &mut BindingBuilder, handle: SamplerHandle) {
    if self.bindless.is_some() {
      return;
    }
    self.base.bind_sampler(binding, handle)
  }

  pub fn bind_system(&self, binding: &mut BindingBuilder) {
    if let Some(bindless) = &self.bindless {
      bindless.bind_system_self(binding);
    }
  }

  pub fn shader_bind_sampler(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: SamplerHandle,
  ) -> HandleNode<ShaderSampler> {
    self.base.register_shader_sampler(builder, handle)
  }

  pub fn shader_bind_texture(
    &self,
    builder: &mut ShaderBindGroupDirectBuilder,
    handle: Texture2DHandle,
  ) -> HandleNode<ShaderTexture2D> {
    self.base.register_shader_texture2d(builder, handle)
  }

  pub fn shader_system(&self, builder: &mut ShaderRenderPipelineBuilder) {
    if let Some(bindless) = &self.bindless {
      bindless.register_system_self(builder);
    }
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
    if let Some(bindless) = &self.bindless {
      bindless.sample_texture2d_indirect(reg, device_handles.0, device_handles.1, uv)
    } else {
      let texture = self.shader_bind_texture(binding, host_handles.0);
      let sampler = self.shader_bind_sampler(binding, host_handles.1);
      texture.sample(sampler, uv)
    }
  }
}
