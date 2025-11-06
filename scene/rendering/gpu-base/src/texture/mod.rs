mod cube;
use std::sync::Arc;

pub use cube::*;
mod d2_and_sampler;
pub use d2_and_sampler::*;
use fast_hash_collection::FastHashMap;
use parking_lot::RwLock;
use rendiation_texture_packer::pack_2d_to_3d::RemappedGrowablePacker;

use crate::*;

pub fn use_texture_system(
  cx: &mut QueryGPUHookCx,
  ty: GPUTextureBindingSystemType,
  pool_init_config: &TexturePoolSourceInit,
) -> Option<GPUTextureBindingSystem> {
  match ty {
    GPUTextureBindingSystemType::GlesSingleBinding => cx.scope(|cx| use_gles_texture_system(cx)),
    GPUTextureBindingSystemType::Bindless => cx.scope(|cx| use_bindless_texture_system(cx)),
    GPUTextureBindingSystemType::TexturePool => {
      cx.scope(|cx| use_pool_texture_system(cx, pool_init_config))
    }
  }
}

fn create_default_tex_and_sampler(
  gpu: &GPU,
  _: &dyn AbstractStorageAllocator,
) -> (GPU2DTextureView, GPUSamplerView) {
  let default_2d: GPU2DTextureView = create_fallback_empty_texture(&gpu.device)
    .create_default_view()
    .try_into()
    .unwrap();
  let default_sampler = create_gpu_sampler(gpu, &TextureSampler::default());
  (default_2d, default_sampler)
}

pub fn use_gles_texture_system(cx: &mut QueryGPUHookCx) -> Option<GPUTextureBindingSystem> {
  let (cx, (default_2d, default_sampler)) = cx.use_gpu_init(create_default_tex_and_sampler);
  let textures = use_gpu_texture_2ds(cx, default_2d);
  let samplers = use_sampler_gpus(cx);

  cx.when_render(|| {
    Box::new(TraditionalPerDrawBindingSystem {
      textures: textures.0.into_boxed(),
      samplers: samplers.0.into_boxed(),
      default_tex: default_2d.clone(),
      default_sampler: default_sampler.clone(),
    }) as GPUTextureBindingSystem
  })
}

pub fn use_bindless_texture_system(cx: &mut QueryGPUHookCx) -> Option<GPUTextureBindingSystem> {
  let (cx, (default_2d, default_sampler)) = cx.use_gpu_init(create_default_tex_and_sampler);

  let bindless_minimal_effective_count = BINDLESS_EFFECTIVE_COUNT;

  let (cx, bindless_texture_2d) = cx.use_plain_state(|| {
    BindingArrayMaintainer::new(default_2d.clone(), bindless_minimal_effective_count)
  });

  let (textures, changed) = use_gpu_texture_2ds(cx, default_2d);
  if changed {
    bindless_texture_2d.update(textures, cx.gpu);
  }

  let (cx, bindless_samplers) = cx.use_plain_state(|| {
    BindingArrayMaintainer::new(default_sampler.clone(), bindless_minimal_effective_count)
  });

  let (samplers, changed) = use_sampler_gpus(cx);
  if changed {
    bindless_samplers.update(samplers, cx.gpu);
  }

  cx.when_render(|| {
    Box::new(BindlessTextureSystem {
      texture_binding_array: bindless_texture_2d.get_gpu(),
      sampler_binding_array: bindless_samplers.get_gpu(),
    }) as GPUTextureBindingSystem
  })
}

pub fn use_pool_texture_system(
  cx: &mut QueryGPUHookCx,
  init: &TexturePoolSourceInit,
) -> Option<GPUTextureBindingSystem> {
  let (cx, samplers) =
    cx.use_storage_buffer("sampler info", init.init_sampler_count_capacity, u32::MAX);
  cx.use_changes::<SceneSamplerInfo>()
    .map_changes(TextureSamplerShaderInfo::from)
    .update_storage_array(cx, samplers, 0);

  samplers.use_max_item_count_by_db_entity::<SceneSamplerEntity>(cx);
  samplers.use_update(cx);

  let (cx, texture_address) = cx.use_storage_buffer(
    "texture_address info",
    init.init_texture_count_capacity,
    u32::MAX,
  );

  let require_convert = cx.gpu.info().adaptor_info.backend != Backend::Gl;
  cx.use_changes::<SceneTexture2dEntityDirectContent>()
    .map_changes(move |v| {
      v.map(|v| {
        if require_convert {
          Bool::from(v.format.is_srgb())
        } else {
          Bool::from(false)
        }
      })
      .unwrap_or_default()
    })
    .update_storage_array(
      cx,
      texture_address,
      offset_of!(TexturePoolTextureMeta, require_srgb_to_linear_convert),
    );

  let (cx, atlas) = cx.use_plain_state_default::<Arc<RwLock<Option<GPU2DArrayTextureView>>>>();

  // todo, spawn a task to pack
  let (cx, packer) = cx.use_sharable_plain_state(|| RemappedGrowablePacker::new(init.atlas_config));

  let content_changes = cx
    .use_changes::<SceneTexture2dEntityDirectContent>()
    .filter_map_changes(|v| v.map(|v| v.ptr));

  let gpu = cx.gpu.clone();
  let packer = packer.clone();
  let _atlas = atlas.clone();

  cx.use_changes::<SceneTexture2dEntityDirectContent>()
    .filter_map_changes(|tex| tex.map(|tex| tex.size))
    .map_only_spawn_stage(move |size_changes| {
      let content_view = get_db_view_uncheck_access::<SceneTexture2dEntityDirectContent>();
      let content_changes = content_changes.into_spawn_stage_ready();
      let mut buff_changes = FastHashMap::default();

      let mut packer = packer.write();
      packer.process(
        size_changes.iter_removed(),
        size_changes.iter_update_or_insert(),
        |_new_size| {},
        |key, delta| {
          merge_change(&mut buff_changes, (key, delta));
        },
      );

      update_atlas(
        &gpu,
        &mut _atlas.write(),
        TEXTURE_POOL_FORMAT,
        |id| packer.access(&id).unwrap(),
        buff_changes.clone().into_iter(),
        |id| content_view.access(&id).unwrap().unwrap().ptr,
        content_changes
          .map(|change| change.iter_update_or_insert().collect::<Vec<_>>()) // todo, bad
          .into_iter()
          .flatten(),
        packer.current_size(),
      );

      buff_changes
        .into_change()
        .collective_map(TexturePoolTextureMetaLayoutInfo::from)
    })
    .update_storage_array(cx, texture_address, 0);

  texture_address.use_max_item_count_by_db_entity::<SceneTexture2dEntity>(cx);
  texture_address.use_update(cx);

  cx.when_render(|| {
    Box::new(TexturePool {
      texture: atlas.read().clone().unwrap(),
      address: texture_address.get_gpu_buffer(),
      samplers: samplers.get_gpu_buffer(),
    }) as GPUTextureBindingSystem
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
  if require_indirect {
    if prefer_bindless && is_bindless_supported_on_this_gpu(&cx.info, BINDLESS_EFFECTIVE_COUNT) {
      GPUTextureBindingSystemType::Bindless
    } else {
      GPUTextureBindingSystemType::TexturePool
    }
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
