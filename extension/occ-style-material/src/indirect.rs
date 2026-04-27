use std::any::Any;

use rendiation_scene_rendering_gpu_indirect::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

use crate::*;

pub fn use_occ_material_storage(
  cx: &mut QueryGPUHookCx,
) -> Option<OccStyleMaterialIndirectRenderer> {
  let (cx, storages) = cx.use_storage_buffer("occ material parameter data", 128, u32::MAX);

  cx.use_changes::<OccStyleMaterialDiffuse>()
    .map_changes(srgb4_to_linear4)
    .update_storage_array(cx, storages, offset_of!(OccStyleMaterialStorage, diffuse));

  cx.use_changes::<OccStyleMaterialSpecular>()
    .update_storage_array(cx, storages, offset_of!(OccStyleMaterialStorage, specular));

  cx.use_changes::<OccStyleMaterialShiness>()
    .update_storage_array(cx, storages, offset_of!(OccStyleMaterialStorage, shiness));

  cx.use_changes::<OccStyleMaterialEmissive>()
    .update_storage_array(cx, storages, offset_of!(OccStyleMaterialStorage, emissive));

  storages.use_max_item_count_by_db_entity::<OccStyleMaterialEntity>(cx);
  storages.use_update(cx);

  let (cx, tex_storages) = cx.use_storage_buffer("occ material texture data", 128, u32::MAX);

  let diffuse_tex = offset_of!(TexStorage, diffuse_texture);
  use_tex_watcher::<OccStyleMaterialDiffuseTex, _>(cx, tex_storages, diffuse_tex);

  tex_storages.use_max_item_count_by_db_entity::<OccStyleMaterialEntity>(cx);
  tex_storages.use_update(cx);

  cx.when_render(|| OccStyleMaterialIndirectRenderer {
    material_access: read_global_db_foreign_key(),
    transparent: read_global_db_component(),
    effect_access: read_global_db_foreign_key(),
    shade_type: read_global_db_component(),
    storages: storages.get_gpu_buffer(),
    texture_handles: tex_storages.get_gpu_buffer(),
  })
}

#[derive(Clone)]
pub struct OccStyleMaterialIndirectRenderer {
  material_access: ForeignKeyReadView<StdModelOccStyleMaterialPayload>,
  transparent: ComponentReadView<OccStyleMaterialTransparent>,
  effect_access: ForeignKeyReadView<OccStyleMaterialEffect>,
  shade_type: ComponentReadView<OccStyleEffectShadeType>,
  storages: AbstractReadonlyStorageBuffer<[OccStyleMaterialStorage]>,
  texture_handles: AbstractReadonlyStorageBuffer<[OccStyleMaterialTextureHandlesStorage]>,
}

impl IndirectModelMaterialRenderImpl for OccStyleMaterialIndirectRenderer {
  fn make_component_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let m = self.material_access.get(any_idx)?;
    let transparent = self.transparent.get_value(m)?;
    let effect = self.effect_access.get(m)?;
    let shade_type = self.shade_type.get_value(effect)?;
    Some(Box::new(OccStyleMaterialStorageGPU {
      buffer: self.storages.clone(),
      texture_handles: self.texture_handles.clone(),
      binding_sys: cx,
      transparent,
      shade_type,
    }))
  }

  fn hash_shader_group_key(
    &self,
    any_idx: EntityHandle<StandardModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    let m = self.material_access.get(any_idx)?;
    let transparent = self.transparent.get_value(m)?;
    let effect = self.effect_access.get(m)?;
    let shade_type = self.shade_type.get_value(effect)?;
    transparent.hash(hasher);
    shade_type.hash(hasher);
    Some(())
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}

type TexStorage = OccStyleMaterialTextureHandlesStorage;

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Default)]
pub struct OccStyleMaterialStorage {
  pub diffuse: Vec4<f32>,
  pub specular: Vec3<f32>,
  pub shiness: f32,
  pub emissive: Vec3<f32>,
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
pub struct OccStyleMaterialTextureHandlesStorage {
  pub diffuse_texture: TextureSamplerHandlePair,
}

#[derive(Clone)]
pub struct OccStyleMaterialStorageGPU<'a> {
  pub buffer: AbstractReadonlyStorageBuffer<[OccStyleMaterialStorage]>,
  pub texture_handles: AbstractReadonlyStorageBuffer<[OccStyleMaterialTextureHandlesStorage]>,
  pub binding_sys: &'a GPUTextureBindingSystem,
  pub transparent: bool,
  pub shade_type: OccStyleEffectType,
}

impl ShaderHashProvider for OccStyleMaterialStorageGPU<'_> {
  shader_hash_type_id! {OccStyleMaterialStorageGPU<'static>}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.transparent.hash(hasher);
    self.shade_type.hash(hasher);
  }
}

impl GraphicsShaderProvider for OccStyleMaterialStorageGPU<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, _| {
      if self.transparent {
        builder.frag_output.iter_mut().for_each(|p| {
          if p.is_blendable() {
            p.states.blend = BlendState::ALPHA_BLENDING.into();
          }
        });
      }
    });
  }
}

impl ShaderPassBuilder for OccStyleMaterialStorageGPU<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.buffer);
    ctx.binding.bind(&self.texture_handles);
  }
}
