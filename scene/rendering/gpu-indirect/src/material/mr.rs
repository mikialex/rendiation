use rendiation_lighting_transport::{EmissiveChannel, MetallicChannel, RoughnessChannel};
use rendiation_shader_library::normal_mapping::apply_normal_mapping_conditional;

use crate::*;

pub fn use_pbr_mr_material_storage(
  cx: &mut QueryGPUHookCx,
) -> Option<PbrMRMaterialIndirectRenderer> {
  let (cx, storages) = cx.use_storage_buffer(128, u32::MAX);

  cx.use_changes::<PbrMRMaterialBaseColorComponent>()
    .update_storage_array(storages, offset_of!(Storage, base_color));

  cx.use_changes::<PbrMRMaterialEmissiveComponent>()
    .update_storage_array(storages, offset_of!(Storage, emissive));

  cx.use_changes::<NormalScaleOf<PbrMRMaterialNormalInfo>>()
    .update_storage_array(storages, offset_of!(Storage, normal_mapping_scale));

  cx.use_changes::<PbrMRMaterialRoughnessComponent>()
    .update_storage_array(storages, offset_of!(Storage, roughness));

  cx.use_changes::<PbrMRMaterialMetallicComponent>()
    .update_storage_array(storages, offset_of!(Storage, metallic));

  cx.use_changes::<AlphaOf<PbrMRMaterialAlphaConfig>>()
    .update_storage_array(storages, offset_of!(Storage, alpha));

  let (cx, tex_storages) = cx.use_storage_buffer(128, u32::MAX);

  let base_color_alpha = offset_of!(TexStorage, base_color_alpha_texture);
  let emissive = offset_of!(TexStorage, emissive_texture);
  let metallic_roughness = offset_of!(TexStorage, metallic_roughness_texture);
  let normal = offset_of!(TexStorage, normal_texture);

  use_tex_watcher::<PbrMRMaterialBaseColorAlphaTex, _>(cx, tex_storages, base_color_alpha);
  use_tex_watcher::<PbrMRMaterialEmissiveTex, _>(cx, tex_storages, emissive);
  use_tex_watcher::<PbrMRMaterialMetallicRoughnessTex, _>(cx, tex_storages, metallic_roughness);
  use_tex_watcher::<NormalTexSamplerOf<PbrMRMaterialNormalInfo>, _>(cx, tex_storages, normal);

  cx.when_render(|| PbrMRMaterialIndirectRenderer {
    material_access: global_entity_component_of::<StandardModelRefPbrMRMaterial>()
      .read_foreign_key(),
    storages: storages.get_gpu_buffer(),
    tex_storages: tex_storages.get_gpu_buffer(),
    alpha_mode: global_entity_component_of().read(),
  })
}

#[derive(Clone)]
pub struct PbrMRMaterialIndirectRenderer {
  material_access: ForeignKeyReadView<StandardModelRefPbrMRMaterial>,
  pub storages: StorageBufferReadonlyDataView<[PhysicalMetallicRoughnessMaterialStorage]>,
  pub tex_storages:
    StorageBufferReadonlyDataView<[PhysicalMetallicRoughnessMaterialTextureHandlesStorage]>,
  alpha_mode: ComponentReadView<AlphaModeOf<PbrMRMaterialAlphaConfig>>,
}

impl IndirectModelMaterialRenderImpl for PbrMRMaterialIndirectRenderer {
  fn make_component_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let idx = self.material_access.get(any_idx)?;
    let r = PhysicalMetallicRoughnessMaterialIndirectGPU {
      storage: &self.storages,
      alpha_mode: self.alpha_mode.get_value(idx)?,
      texture_storages: &self.tex_storages,
      binding_sys: cx,
    };
    let r = Box::new(r) as Box<dyn RenderComponent + '_>;
    Some(r)
  }
  fn hash_shader_group_key(
    &self,
    idx: EntityHandle<StandardModelEntity>,
    hasher: &mut PipelineHasher,
  ) -> Option<()> {
    let idx = self.material_access.get(idx)?;
    self.alpha_mode.get_value(idx)?.hash(hasher);
    Some(())
  }
  fn as_any(&self) -> &dyn Any {
    self
  }
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
pub struct PhysicalMetallicRoughnessMaterialStorage {
  pub base_color: Vec3<f32>,
  pub emissive: Vec3<f32>,
  pub roughness: f32,
  pub metallic: f32,
  pub reflectance: f32,
  pub normal_mapping_scale: f32,
  pub alpha_cutoff: f32,
  pub alpha: f32,
}

type Storage = PhysicalMetallicRoughnessMaterialStorage;

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
pub struct PhysicalMetallicRoughnessMaterialTextureHandlesStorage {
  pub base_color_alpha_texture: TextureSamplerHandlePair,
  pub emissive_texture: TextureSamplerHandlePair,
  pub metallic_roughness_texture: TextureSamplerHandlePair,
  pub normal_texture: TextureSamplerHandlePair,
}

type TexStorage = PhysicalMetallicRoughnessMaterialTextureHandlesStorage;

pub struct PhysicalMetallicRoughnessMaterialIndirectGPU<'a> {
  storage: &'a StorageBufferReadonlyDataView<[PhysicalMetallicRoughnessMaterialStorage]>,
  alpha_mode: AlphaMode,
  texture_storages:
    &'a StorageBufferReadonlyDataView<[PhysicalMetallicRoughnessMaterialTextureHandlesStorage]>,
  binding_sys: &'a GPUTextureBindingSystem,
}

impl ShaderHashProvider for PhysicalMetallicRoughnessMaterialIndirectGPU<'_> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.alpha_mode.hash(hasher);
  }
  shader_hash_type_id! {PhysicalMetallicRoughnessMaterialIndirectGPU<'static>}
}

impl ShaderPassBuilder for PhysicalMetallicRoughnessMaterialIndirectGPU<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.storage);
    ctx.binding.bind(self.texture_storages);
  }
}

impl GraphicsShaderProvider for PhysicalMetallicRoughnessMaterialIndirectGPU<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.fragment(|builder, binding| {
      let id = builder.query::<IndirectAbstractMaterialId>();
      let storage = binding.bind_by(&self.storage).index(id).load().expand();
      let tex_storage = binding
        .bind_by(&self.texture_storages)
        .index(id)
        .load()
        .expand();

      let uv = builder.query_or_interpolate_by::<FragmentUv, GeometryUV>();

      let mut alpha = storage.alpha;
      let mut base_color = storage.base_color;

      let base_color_alpha_tex = indirect_sample(
        self.binding_sys,
        builder.registry(),
        tex_storage.base_color_alpha_texture,
        uv,
        val(Vec4::one()),
      );
      alpha *= base_color_alpha_tex.w();
      base_color *= base_color_alpha_tex.xyz();

      let mut metallic = storage.metallic;
      let mut roughness = storage.roughness;

      let metallic_roughness_tex = indirect_sample(
        self.binding_sys,
        builder.registry(),
        tex_storage.metallic_roughness_texture,
        uv,
        val(Vec4::one()),
      );

      metallic *= metallic_roughness_tex.x();
      roughness *= metallic_roughness_tex.y();

      let mut emissive = storage.emissive;
      emissive *= indirect_sample(
        self.binding_sys,
        builder.registry(),
        tex_storage.emissive_texture,
        uv,
        val(Vec4::one()),
      )
      .xyz();

      let (normal_sample, enabled) = indirect_sample_enabled(
        self.binding_sys,
        builder.registry(),
        tex_storage.normal_texture,
        uv,
      );

      apply_normal_mapping_conditional(
        builder,
        normal_sample.xyz(),
        uv,
        storage.normal_mapping_scale,
        enabled,
      );

      ShaderAlphaConfig {
        alpha_mode: self.alpha_mode,
        alpha_cutoff: storage.alpha_cutoff,
        alpha,
      }
      .apply(builder);

      builder.register::<ColorChannel>(base_color);
      builder.register::<EmissiveChannel>(emissive);
      builder.register::<MetallicChannel>(metallic);
      builder.register::<RoughnessChannel>(roughness * roughness);

      builder.register::<DefaultDisplay>((base_color, val(1.)));
      builder.insert_type_tag::<PbrMRMaterialTag>();
      builder.insert_type_tag::<LightableSurfaceTag>();
    })
  }
}
