use rendiation_lighting_transport::{EmissiveChannel, GlossinessChannel, SpecularChannel};
use rendiation_shader_library::normal_mapping::apply_normal_mapping_conditional;

use crate::*;

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

pub type PbrSGMaterialStorages = ReactiveStorageBufferContainer<Storage>;
pub fn pbr_sg_material_storages(cx: &GPU) -> PbrSGMaterialStorages {
  let albedo = global_watch().watch::<PbrSGMaterialAlbedoComponent>();
  let albedo_offset = offset_of!(Storage, albedo);

  let emissive = global_watch().watch::<PbrSGMaterialEmissiveComponent>();
  let emissive_offset = offset_of!(Storage, emissive);

  let normal_mapping_scale = global_watch().watch::<NormalScaleOf<PbrSGMaterialNormalInfo>>();
  let normal_mapping_scale_offset = offset_of!(Storage, normal_mapping_scale);

  let glossiness = global_watch().watch::<PbrSGMaterialGlossinessComponent>();
  let glossiness_offset = offset_of!(Storage, glossiness);

  let alpha = global_watch().watch::<AlphaOf<PbrSGMaterialAlphaConfig>>();
  let alpha_offset = offset_of!(Storage, alpha);

  PbrSGMaterialStorages::new(cx)
    .with_source(albedo, albedo_offset)
    .with_source(emissive, emissive_offset)
    .with_source(normal_mapping_scale, normal_mapping_scale_offset)
    .with_source(glossiness, glossiness_offset)
    .with_source(alpha, alpha_offset)
}

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

pub type PbrSGMaterialTexStorages = ReactiveStorageBufferContainer<TexStorage>;
pub fn pbr_sg_material_tex_storages(cx: &GPU) -> PbrSGMaterialTexStorages {
  let c = PbrSGMaterialTexStorages::new(cx);

  let albedo = offset_of!(TexStorage, albedo_texture);
  let emissive = offset_of!(TexStorage, emissive_texture);
  let specular_glossiness = offset_of!(TexStorage, specular_glossiness_texture);
  let normal = offset_of!(TexStorage, normal_texture);
  let c = add_tex_watcher::<PbrSGMaterialAlbedoAlphaTex, _>(c, albedo);
  let c = add_tex_watcher::<PbrSGMaterialEmissiveTex, _>(c, emissive);
  let c = add_tex_watcher::<PbrSGMaterialSpecularGlossinessTex, _>(c, specular_glossiness);
  add_tex_watcher::<NormalTexSamplerOf<PbrSGMaterialNormalInfo>, _>(c, normal)
}

pub fn pbr_sg_material_pipeline_hash(
) -> impl ReactiveQuery<Key = EntityHandle<PbrSGMaterialEntity>, Value = AlphaMode> {
  global_watch().watch::<AlphaModeOf<PbrSGMaterialAlphaConfig>>()
}

pub struct PhysicalSpecularGlossinessMaterialGPU<'a> {
  pub storage: &'a StorageBufferReadOnlyDataView<[PhysicalSpecularGlossinessMaterialStorage]>,
  pub alpha_mode: AlphaMode,
  // no matter if we using indirect texture binding, this storage is required for checking the
  // texture if is exist in shader
  pub texture_storages:
    &'a StorageBufferReadOnlyDataView<[PhysicalSpecularGlossinessMaterialTextureHandlesStorage]>,
  pub binding_sys: &'a GPUTextureBindingSystem,
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

      let albedo = bind_and_sample(
        self.binding_sys,
        builder.registry(),
        tex_storage.albedo_texture,
        uv,
        val(Vec4::one()),
      );
      alpha *= albedo.w();
      base_color *= albedo.xyz();

      let mut specular = storage.specular;
      let specular_glossiness = bind_and_sample(
        self.binding_sys,
        builder.registry(),
        tex_storage.specular_glossiness_texture,
        uv,
        val(Vec4::one()),
      );
      specular *= specular_glossiness.xyz();

      let glossiness = storage.glossiness * specular_glossiness.w();

      let mut emissive = storage.emissive;
      emissive *= bind_and_sample(
        self.binding_sys,
        builder.registry(),
        tex_storage.emissive_texture,
        uv,
        val(Vec4::one()),
      )
      .xyz();

      let (normal_sample, enabled) = bind_and_sample_enabled(
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
      builder.register::<GlossinessChannel>(glossiness);

      builder.register::<DefaultDisplay>((albedo.xyz(), val(1.)));
      builder.insert_type_tag::<PbrSGMaterialTag>();
    })
  }
}
