mod cube;
use std::sync::Arc;

pub use cube::*;
mod d2_and_sampler;
pub use d2_and_sampler::*;
use fast_hash_collection::FastHashMap;
use parking_lot::RwLock;
use rendiation_texture_packer::pack_2d_to_3d::RemappedGrowablePacker;

use crate::*;

pub fn use_texture_system<
  R: DataChanges<Key = u32, Value = Option<Arc<GPUBufferImage>>> + 'static,
>(
  cx: &mut QueryGPUHookCx,
  ty: GPUTextureBindingSystemType,
  pool_init_config: &TexturePoolSourceInit,
  source_creator: impl FnOnce(&mut QueryGPUHookCx<'_>) -> UseResult<R>,
) -> Option<GPUTextureBindingSystem> {
  // note, we must create source for each scope because if somehow we changed system type,
  // we need the source emit new inits messages
  match ty {
    GPUTextureBindingSystemType::GlesSingleBinding => cx.scope(|cx| {
      let source = source_creator(cx);
      use_gles_texture_system(cx, source)
    }),
    GPUTextureBindingSystemType::Bindless => cx.scope(|cx| {
      let source = source_creator(cx);
      use_bindless_texture_system(cx, source)
    }),
    GPUTextureBindingSystemType::TexturePool => cx.scope(|cx| {
      let source = source_creator(cx);
      use_pool_texture_system(cx, pool_init_config, source)
    }),
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

pub fn use_gles_texture_system(
  cx: &mut QueryGPUHookCx,
  source: UseResult<impl DataChanges<Key = u32, Value = Option<Arc<GPUBufferImage>>> + 'static>,
) -> Option<GPUTextureBindingSystem> {
  let (cx, (default_2d, default_sampler)) = cx.use_gpu_init(create_default_tex_and_sampler);
  let textures = use_gpu_texture_2ds(cx, default_2d, source);
  let samplers = use_sampler_gpus(cx);

  cx.when_render(|| {
    Box::new(TraditionalPerDrawBindingSystem {
      textures: textures.into_boxed(),
      samplers: samplers.into_boxed(),
      default_tex: default_2d.clone(),
      default_sampler: default_sampler.clone(),
    }) as GPUTextureBindingSystem
  })
}

pub fn use_bindless_texture_system(
  cx: &mut QueryGPUHookCx,
  source: UseResult<impl DataChanges<Key = u32, Value = Option<Arc<GPUBufferImage>>> + 'static>,
) -> Option<GPUTextureBindingSystem> {
  let (cx, (default_2d, default_sampler)) = cx.use_gpu_init(create_default_tex_and_sampler);

  let bindless_minimal_effective_count = BINDLESS_EFFECTIVE_COUNT;

  let (cx, bindless_texture_2d) = cx.use_plain_state(|| {
    BindingArrayMaintainer::new(default_2d.clone(), bindless_minimal_effective_count)
  });

  let (textures, changed) =
    cx.run_with_waked_info(|cx, _| use_gpu_texture_2ds(cx, default_2d, source));
  if changed {
    bindless_texture_2d.update(textures, cx.gpu);
  }

  let (cx, bindless_samplers) = cx.use_plain_state(|| {
    BindingArrayMaintainer::new(default_sampler.clone(), bindless_minimal_effective_count)
  });

  let (samplers, changed) = cx.run_with_waked_info(|cx, _| use_sampler_gpus(cx));
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
  source: UseResult<impl DataChanges<Key = u32, Value = Option<Arc<GPUBufferImage>>> + 'static>,
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

  let (source, source_) = source.fork();
  let (source_, source__) = source_.fork();

  let require_convert = cx.gpu.info().adaptor_info.backend != Backend::Gl;
  source
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
  let (cx, packer) = cx.use_sharable_plain_state(|| RemappedGrowablePacker::new(init.atlas_config));

  let gpu = cx.gpu.clone();
  let packer_ = packer.clone();
  let _atlas = atlas.clone();

  let packing_changes = source_
    .filter_map_changes(|tex| tex.map(|tex| tex.size))
    .map_spawn_stage_in_thread(
      cx,
      |changes| changes.has_change(),
      move |size_changes| {
        let mut packing_changes = FastHashMap::default();

        let mut packer = packer_.write();
        packer.process(
          size_changes.iter_removed(),
          size_changes.iter_update_or_insert(),
          |_new_size| {},
          |key, delta| {
            merge_change(&mut packing_changes, (key, delta));
          },
        );
        Arc::new(packing_changes)
      },
    );

  let (packing_changes, packing_changes_) = packing_changes.fork();

  packing_changes
    .map(|v| {
      v.into_change()
        .collective_map(TexturePoolTextureMetaLayoutInfo::from)
    })
    .update_storage_array(cx, texture_address, 0);

  let content_changes = source__.use_assure_result(cx);

  let packing_changes_ = packing_changes_.use_assure_result(cx);
  if let GPUQueryHookStage::CreateRender { encoder, .. } = &mut cx.stage {
    let packer = packer.write();
    let packing_changes = packing_changes_.into_resolve_stage();
    let content_changes = content_changes.into_resolve_stage();

    let content_changes = content_changes
      .map(|change| change.iter_update_or_insert().collect::<Vec<_>>())
      .unwrap_or_default(); // todo, bad

    let iter = content_changes.iter().filter_map(|(k, v)| {
      let v = v.as_ref()?;
      Some((*k, v.as_ref()))
    });

    update_atlas(
      &gpu,
      encoder,
      &mut _atlas.write(),
      TEXTURE_POOL_FORMAT,
      packer.current_size(),
      packing_changes
        .map(|v| (v.as_ref().clone()).into_iter())
        .into_iter()
        .flatten(), // todo, bad
      iter,
      |id| packer.access(&id).unwrap(),
    );
  }

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
