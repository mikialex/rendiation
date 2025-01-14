mod cube;
use std::num::NonZeroU32;

pub use cube::*;

mod d2_and_sampler;
pub use d2_and_sampler::*;

use crate::*;

const BINDLESS_EFFECTIVE_COUNT: u32 = 8192;

pub fn get_suitable_texture_system_ty(
  cx: &GPU,
  require_indirect: bool,
  prefer_bindless: bool,
) -> GPUTextureBindingSystemType {
  if prefer_bindless && is_bindless_supported_on_this_gpu(&cx.info, BINDLESS_EFFECTIVE_COUNT) {
    GPUTextureBindingSystemType::Bindless
  } else if require_indirect {
    GPUTextureBindingSystemType::TexturePool
  } else {
    GPUTextureBindingSystemType::GlesSingleBinding
  }
}

pub enum GPUTextureBindingSystemType {
  GlesSingleBinding,
  Bindless,
  TexturePool,
}

pub struct TextureGPUSystemSource {
  pub token: UpdateResultToken,
  pub ty: GPUTextureBindingSystemType,
}

impl TextureGPUSystemSource {
  pub fn new(ty: GPUTextureBindingSystemType) -> Self {
    Self {
      token: Default::default(),
      ty,
    }
  }
}

impl TextureGPUSystemSource {
  pub fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    match self.ty {
      GPUTextureBindingSystemType::GlesSingleBinding => {
        let default_2d: GPU2DTextureView = create_fallback_empty_texture(&cx.device)
          .create_default_view()
          .try_into()
          .unwrap();
        let texture_2d = gpu_texture_2ds(cx, default_2d.clone());

        let default_sampler = create_gpu_sampler(cx, &TextureSampler::default());
        let samplers = sampler_gpus(cx);

        let texture_system = TraditionalPerDrawBindingSystemSource {
          default_tex: default_2d,
          default_sampler,
          textures: Box::new(texture_2d),
          samplers: Box::new(samplers),
        };
        self.token = source.register(Box::new(ReactiveQueryBoxAnyResult(texture_system)));
      }
      GPUTextureBindingSystemType::Bindless => {
        let default_2d: GPU2DTextureView = create_fallback_empty_texture(&cx.device)
          .create_default_view()
          .try_into()
          .unwrap();
        let texture_2d = gpu_texture_2ds(cx, default_2d.clone());

        let default_sampler = create_gpu_sampler(cx, &TextureSampler::default());
        let samplers = sampler_gpus(cx);

        let bindless_minimal_effective_count = BINDLESS_EFFECTIVE_COUNT;
        let texture_system = BindlessTextureSystemSource::new(
          texture_2d,
          default_2d,
          samplers,
          default_sampler,
          bindless_minimal_effective_count,
        );

        self.token = source.register(Box::new(ReactiveQueryBoxAnyResult(texture_system)));
      }
      GPUTextureBindingSystemType::TexturePool => {
        let samplers = global_watch().watch_untyped_key::<SceneSamplerInfo>();
        let texture_2d = global_watch()
          .watch_untyped_key::<SceneTexture2dEntityDirectContent>()
          .collective_filter_map(|v| {
            v.map(|v| TexturePool2dSource {
              inner: v.ptr.clone(),
            })
          })
          .into_boxed();

        let size = Size::from_u32_pair_min_one((2048, 2048));

        let texture_system = TexturePoolSource::new(
          cx,
          MultiLayerTexturePackerConfig {
            max_size: SizeWithDepth {
              depth: NonZeroU32::new(4).unwrap(),
              size,
            },
            init_size: SizeWithDepth {
              depth: NonZeroU32::new(1).unwrap(),
              size,
            },
          },
          texture_2d.into_forker(),
          Box::new(samplers),
          TextureFormat::Rgba8Unorm,
        );

        self.token = source.register(Box::new(ReactiveQueryBoxAnyResult(texture_system)));
      }
    }
  }

  pub fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.token);
  }

  pub fn create_impl(&self, res: &mut QueryResultCtx) -> GPUTextureBindingSystem {
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
