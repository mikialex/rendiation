use rendiation_lighting_transport::{EmissiveChannel, GlossinessChannel, SpecularChannel};
use rendiation_shader_library::normal_mapping::apply_normal_mapping_conditional;

use crate::*;

pub fn use_pbr_sg_material_storage(
  cx: &mut QueryGPUHookCx,
) -> Option<PbrSGMaterialIndirectRenderer> {
  let (cx, storages) = cx.use_storage_buffer2(128, u32::MAX);

  cx.use_changes::<PbrSGMaterialAlbedoComponent>()
    .update_storage_array(storages, offset_of!(Storage, albedo));

  cx.use_changes::<PbrSGMaterialEmissiveComponent>()
    .update_storage_array(storages, offset_of!(Storage, emissive));

  cx.use_changes::<NormalScaleOf<PbrSGMaterialNormalInfo>>()
    .update_storage_array(storages, offset_of!(Storage, normal_mapping_scale));

  cx.use_changes::<PbrSGMaterialGlossinessComponent>()
    .update_storage_array(storages, offset_of!(Storage, glossiness));

  cx.use_changes::<AlphaOf<PbrSGMaterialAlphaConfig>>()
    .update_storage_array(storages, offset_of!(Storage, alpha));

  let (cx, tex_storages) = cx.use_storage_buffer2(128, u32::MAX);

  let albedo = offset_of!(TexStorage, albedo_texture);
  let emissive = offset_of!(TexStorage, emissive_texture);
  let specular_glossiness = offset_of!(TexStorage, specular_glossiness_texture);
  let normal = offset_of!(TexStorage, normal_texture);

  use_tex_watcher::<PbrSGMaterialAlbedoAlphaTex, _>(cx, tex_storages, albedo);
  use_tex_watcher::<PbrSGMaterialEmissiveTex, _>(cx, tex_storages, emissive);
  use_tex_watcher::<PbrSGMaterialSpecularGlossinessTex, _>(cx, tex_storages, specular_glossiness);
  use_tex_watcher::<NormalTexSamplerOf<PbrSGMaterialNormalInfo>, _>(cx, tex_storages, normal);

  cx.when_render(|| PbrSGMaterialIndirectRenderer {
    material_access: global_entity_component_of::<StandardModelRefPbrSGMaterial>()
      .read_foreign_key(),
    storages: storages.get_gpu_buffer(),
    tex_storages: tex_storages.get_gpu_buffer(),
    alpha_mode: global_entity_component_of().read(),
  })
}

pub fn pbr_sg_material_pipeline_hash(
) -> impl ReactiveQuery<Key = EntityHandle<PbrSGMaterialEntity>, Value = AlphaMode> {
  global_watch().watch::<AlphaModeOf<PbrSGMaterialAlphaConfig>>()
}

#[derive(Clone)]
pub struct PbrSGMaterialIndirectRenderer {
  material_access: ForeignKeyReadView<StandardModelRefPbrSGMaterial>,
  pub storages: StorageBufferReadonlyDataView<[PhysicalSpecularGlossinessMaterialStorage]>,
  pub tex_storages:
    StorageBufferReadonlyDataView<[PhysicalSpecularGlossinessMaterialTextureHandlesStorage]>,
  alpha_mode: ComponentReadView<AlphaModeOf<PbrSGMaterialAlphaConfig>>,
}

impl IndirectModelMaterialRenderImpl for PbrSGMaterialIndirectRenderer {
  fn make_component_indirect<'a>(
    &'a self,
    any_idx: EntityHandle<StandardModelEntity>,
    cx: &'a GPUTextureBindingSystem,
  ) -> Option<Box<dyn RenderComponent + 'a>> {
    let idx = self.material_access.get(any_idx)?;
    let r = PhysicalSpecularGlossinessMaterialGPU {
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
pub struct PhysicalSpecularGlossinessMaterialStorage {
  pub albedo: Vec3<f32>,
  pub specular: Vec3<f32>,
  pub emissive: Vec3<f32>,
  pub glossiness: f32,
  pub normal_mapping_scale: f32,
  pub alpha_cutoff: f32,
  pub alpha: f32,
}

type Storage = PhysicalSpecularGlossinessMaterialStorage;

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
pub struct PhysicalSpecularGlossinessMaterialTextureHandlesStorage {
  pub albedo_texture: TextureSamplerHandlePair,
  pub specular_glossiness_texture: TextureSamplerHandlePair,
  pub emissive_texture: TextureSamplerHandlePair,
  pub normal_texture: TextureSamplerHandlePair,
}

type TexStorage = PhysicalSpecularGlossinessMaterialTextureHandlesStorage;

pub struct PhysicalSpecularGlossinessMaterialGPU<'a> {
  storage: &'a StorageBufferReadonlyDataView<[PhysicalSpecularGlossinessMaterialStorage]>,
  alpha_mode: AlphaMode,
  texture_storages:
    &'a StorageBufferReadonlyDataView<[PhysicalSpecularGlossinessMaterialTextureHandlesStorage]>,
  binding_sys: &'a GPUTextureBindingSystem,
}

impl ShaderHashProvider for PhysicalSpecularGlossinessMaterialGPU<'_> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.alpha_mode.hash(hasher);
  }
  shader_hash_type_id! {PhysicalSpecularGlossinessMaterialGPU<'static>}
}

impl ShaderPassBuilder for PhysicalSpecularGlossinessMaterialGPU<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.storage);
    ctx.binding.bind(self.texture_storages);
  }
}

impl GraphicsShaderProvider for PhysicalSpecularGlossinessMaterialGPU<'_> {
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
      let mut base_color = storage.albedo;

      let albedo = indirect_sample(
        self.binding_sys,
        builder.registry(),
        tex_storage.albedo_texture,
        uv,
        val(Vec4::one()),
      );
      alpha *= albedo.w();
      base_color *= albedo.xyz();

      let mut specular = storage.specular;
      let specular_glossiness = indirect_sample(
        self.binding_sys,
        builder.registry(),
        tex_storage.specular_glossiness_texture,
        uv,
        val(Vec4::one()),
      );
      specular *= specular_glossiness.xyz();

      let glossiness = storage.glossiness * specular_glossiness.w();

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
      builder.register::<SpecularChannel>(specular);
      builder.register::<EmissiveChannel>(emissive);
      builder.register::<GlossinessChannel>(glossiness * glossiness);

      builder.register::<DefaultDisplay>((albedo.xyz(), val(1.)));
      builder.insert_type_tag::<PbrSGMaterialTag>();
      builder.insert_type_tag::<LightableSurfaceTag>();
    })
  }
}
