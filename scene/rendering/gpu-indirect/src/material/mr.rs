use rendiation_lighting_transport::{EmissiveChannel, MetallicChannel, RoughnessChannel};
use rendiation_shader_library::normal_mapping::apply_normal_mapping_conditional;

use crate::*;

pub fn use_pbr_mr_material_storage(
  cx: &mut impl QueryGPUHookCx,
) -> Option<PbrMRMaterialIndirectRenderer> {
  let storages = cx.use_storage_buffer(pbr_mr_material_storages);
  let tex_storages = cx.use_storage_buffer(pbr_mr_material_tex_storages);

  cx.when_render(|| PbrMRMaterialIndirectRenderer {
    material_access: global_entity_component_of::<StandardModelRefPbrMRMaterial>()
      .read_foreign_key(),
    storages: storages.unwrap(),
    tex_storages: tex_storages.unwrap(),
    alpha_mode: global_entity_component_of().read(),
  })
}

pub fn pbr_mr_material_pipeline_hash(
) -> impl ReactiveQuery<Key = EntityHandle<PbrMRMaterialEntity>, Value = AlphaMode> {
  global_watch().watch::<AlphaModeOf<PbrMRMaterialAlphaConfig>>()
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
type PbrMRMaterialStorages = ReactiveStorageBufferContainer<Storage>;

fn pbr_mr_material_storages(cx: &GPU) -> PbrMRMaterialStorages {
  let base_color = global_watch()
    .watch::<PbrMRMaterialBaseColorComponent>()
    .into_query_update_storage(offset_of!(Storage, base_color));

  let emissive = global_watch()
    .watch::<PbrMRMaterialEmissiveComponent>()
    .into_query_update_storage(offset_of!(Storage, emissive));

  let normal_mapping_scale = global_watch()
    .watch::<NormalScaleOf<PbrMRMaterialNormalInfo>>()
    .into_query_update_storage(offset_of!(Storage, normal_mapping_scale));

  let roughness = global_watch()
    .watch::<PbrMRMaterialRoughnessComponent>()
    .into_query_update_storage(offset_of!(Storage, roughness));

  let metallic = global_watch()
    .watch::<PbrMRMaterialMetallicComponent>()
    .into_query_update_storage(offset_of!(Storage, metallic));

  let alpha = global_watch()
    .watch::<AlphaOf<PbrMRMaterialAlphaConfig>>()
    .into_query_update_storage(offset_of!(Storage, alpha));

  create_reactive_storage_buffer_container(128, u32::MAX, cx)
    .with_source(base_color)
    .with_source(emissive)
    .with_source(normal_mapping_scale)
    .with_source(roughness)
    .with_source(metallic)
    .with_source(alpha)
}

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
type PbrMRMaterialTexStorages = ReactiveStorageBufferContainer<TexStorage>;

fn pbr_mr_material_tex_storages(cx: &GPU) -> PbrMRMaterialTexStorages {
  let c = create_reactive_storage_buffer_container(128, u32::MAX, cx);

  let base_color_alpha = offset_of!(TexStorage, base_color_alpha_texture);
  let emissive = offset_of!(TexStorage, emissive_texture);
  let metallic_roughness = offset_of!(TexStorage, metallic_roughness_texture);
  let normal = offset_of!(TexStorage, normal_texture);
  let c = add_tex_watcher::<PbrMRMaterialBaseColorAlphaTex, _>(c, base_color_alpha);
  let c = add_tex_watcher::<PbrMRMaterialEmissiveTex, _>(c, emissive);
  let c = add_tex_watcher::<PbrMRMaterialMetallicRoughnessTex, _>(c, metallic_roughness);
  add_tex_watcher::<NormalTexSamplerOf<PbrMRMaterialNormalInfo>, _>(c, normal)
}

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
    })
  }
}
