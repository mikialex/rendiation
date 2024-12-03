mod cube;
pub use cube::*;

mod d2_and_sampler;
pub use d2_and_sampler::*;

use crate::*;

pub struct TextureGPUSystemSource {
  pub token: UpdateResultToken,
  pub prefer_bindless: bool,
}

impl TextureGPUSystemSource {
  pub fn new(prefer_bindless: bool) -> Self {
    Self {
      token: Default::default(),
      prefer_bindless,
    }
  }
}

impl TextureGPUSystemSource {
  pub fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    let default_2d: GPU2DTextureView = create_fallback_empty_texture(&cx.device)
      .create_default_view()
      .try_into()
      .unwrap();
    let texture_2d = gpu_texture_2ds(cx, default_2d.clone());

    let default_sampler = create_gpu_sampler(cx, &TextureSampler::default());
    let samplers = sampler_gpus(cx);

    let bindless_minimal_effective_count = 8192;
    self.token = if self.prefer_bindless
      && is_bindless_supported_on_this_gpu(&cx.info, bindless_minimal_effective_count)
    {
      let texture_system = BindlessTextureSystemSource::new(
        texture_2d,
        default_2d,
        samplers,
        default_sampler,
        bindless_minimal_effective_count,
      );

      source.register(Box::new(ReactiveQueryBoxAnyResult(texture_system)))
    } else {
      let texture_system = TraditionalPerDrawBindingSystemSource {
        default_tex: default_2d,
        default_sampler,
        textures: Box::new(texture_2d),
        samplers: Box::new(samplers),
      };
      source.register(Box::new(ReactiveQueryBoxAnyResult(texture_system)))
    };
  }

  pub fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.token);
  }

  pub fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> GPUTextureBindingSystem {
    *res
      .take_result(self.token)
      .unwrap()
      .downcast::<GPUTextureBindingSystem>()
      .unwrap()
  }
}

#[allow(clippy::borrowed_box)]
pub struct GPUTextureSystemAsRenderComponent<'a>(pub &'a Box<dyn DynAbstractGPUTextureSystem>);

impl<'a> ShaderHashProvider for GPUTextureSystemAsRenderComponent<'a> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.0.hash_pipeline(hasher);
  }
  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    self.0.hash_pipeline_with_type_info(hasher);
  }
}

impl<'a> ShaderPassBuilder for GPUTextureSystemAsRenderComponent<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.0.setup_pass(ctx);
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.0.post_setup_pass(ctx);
  }
}
impl<'a> GraphicsShaderProvider for GPUTextureSystemAsRenderComponent<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.0.build(builder)
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.0.post_build(builder)
  }
}
