mod cube;
use std::num::NonZeroU32;

pub use cube::*;

mod d2_and_sampler;
pub use d2_and_sampler::*;

use crate::*;

pub fn use_texture_system(
  cx: &mut impl QueryGPUHookCx,
  ty: GPUTextureBindingSystemType,
) -> Option<GPUTextureBindingSystem> {
  match ty {
    GPUTextureBindingSystemType::GlesSingleBinding => cx.scope(|cx| use_gles_texture_system(cx)),
    GPUTextureBindingSystemType::Bindless => cx.scope(|cx| use_bindless_texture_system(cx)),
    GPUTextureBindingSystemType::TexturePool => cx.scope(|cx| use_pool_texture_system(cx)),
  }
}

pub fn use_gles_texture_system(cx: &mut impl QueryGPUHookCx) -> Option<GPUTextureBindingSystem> {
  cx.use_gpu_general_query(|cx| {
    let default_2d: GPU2DTextureView = create_fallback_empty_texture(&cx.device)
      .create_default_view()
      .try_into()
      .unwrap();
    let texture_2d = gpu_texture_2ds(cx, default_2d.clone());

    let default_sampler = create_gpu_sampler(cx, &TextureSampler::default());
    let samplers = sampler_gpus(cx);

    TraditionalPerDrawBindingSystemSource {
      default_tex: default_2d,
      default_sampler,
      textures: Box::new(texture_2d),
      samplers: Box::new(samplers),
    }
  })
}

pub fn use_bindless_texture_system(
  cx: &mut impl QueryGPUHookCx,
) -> Option<GPUTextureBindingSystem> {
  cx.use_gpu_general_query(|cx| {
    let default_2d: GPU2DTextureView = create_fallback_empty_texture(&cx.device)
      .create_default_view()
      .try_into()
      .unwrap();
    let texture_2d = gpu_texture_2ds(cx, default_2d.clone());

    let default_sampler = create_gpu_sampler(cx, &TextureSampler::default());
    let samplers = sampler_gpus(cx);

    let bindless_minimal_effective_count = BINDLESS_EFFECTIVE_COUNT;
    BindlessTextureSystemSource::new(
      texture_2d,
      default_2d,
      samplers,
      default_sampler,
      bindless_minimal_effective_count,
    )
  })
}

pub fn use_pool_texture_system(cx: &mut impl QueryGPUHookCx) -> Option<GPUTextureBindingSystem> {
  cx.use_gpu_general_query(|cx| {
    let samplers = global_watch().watch_untyped_key::<SceneSamplerInfo>();
    let texture_2d = global_watch()
      .watch_untyped_key::<SceneTexture2dEntityDirectContent>()
      .collective_map(|v| {
        v.map(|v| TexturePool2dSource {
          inner: v.ptr.clone(),
        })
      })
      .into_boxed();

    let size = Size::from_u32_pair_min_one((4096, 4096));

    TexturePoolSource::new(
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
      texture_2d,
      Box::new(samplers),
      TextureFormat::Rgba8Unorm,
      TexturePoolSourceInit {
        init_texture_count_capacity: 128,
        init_sampler_count_capacity: 128,
      },
    )
  })
}

pub enum GPUTextureBindingSystemType {
  GlesSingleBinding,
  Bindless,
  TexturePool,
}

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

#[allow(clippy::borrowed_box)]
pub struct GPUTextureSystemAsRenderComponent<'a>(pub &'a Box<dyn DynAbstractGPUTextureSystem>);

impl ShaderHashProvider for GPUTextureSystemAsRenderComponent<'_> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.0.hash_pipeline(hasher);
  }
  fn hash_type_info(&self, hasher: &mut PipelineHasher) {
    self.0.hash_pipeline_with_type_info(hasher);
  }
}

impl ShaderPassBuilder for GPUTextureSystemAsRenderComponent<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.0.setup_pass(ctx);
  }

  fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.0.post_setup_pass(ctx);
  }
}
impl GraphicsShaderProvider for GPUTextureSystemAsRenderComponent<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.0.build(builder)
  }

  fn post_build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.0.post_build(builder)
  }
}
