use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
pub struct PhysicalMetallicRoughnessMaterialUniform {
  pub base_color: Vec3<f32>,
  pub emissive: Vec3<f32>,
  pub roughness: f32,
  pub metallic: f32,
  pub reflectance: f32,
  pub normal_mapping_scale: f32,
  pub alpha_cutoff: f32,
  pub alpha: f32,
}
type Uniform = PhysicalMetallicRoughnessMaterialUniform;

pub type PbrMRMaterialUniforms = UniformUpdateContainer<PbrMRMaterialEntity, Uniform>;
pub fn pbr_mr_material_uniforms(cx: &GPUResourceCtx) -> PbrMRMaterialUniforms {
  let base_color = global_watch()
    .watch_typed_key::<PbrMRMaterialBaseColorComponent>()
    .into_uniform_collection_update(offset_of!(Uniform, base_color), cx);

  let emissive = global_watch()
    .watch_typed_key::<PbrMRMaterialEmissiveComponent>()
    .into_uniform_collection_update(offset_of!(Uniform, emissive), cx);

  let normal_mapping_scale = global_watch()
    .watch_typed_key::<NormalScaleOf<PbrMRMaterialNormalInfo>>()
    .into_uniform_collection_update(offset_of!(Uniform, normal_mapping_scale), cx);

  let roughness = global_watch()
    .watch_typed_key::<PbrMRMaterialRoughnessComponent>()
    .into_uniform_collection_update(offset_of!(Uniform, roughness), cx);

  let metallic = global_watch()
    .watch_typed_key::<PbrMRMaterialMetallicComponent>()
    .into_uniform_collection_update(offset_of!(Uniform, metallic), cx);

  let alpha = global_watch()
    .watch_typed_key::<PbrMRMaterialAlphaComponent>()
    .into_uniform_collection_update(offset_of!(Uniform, alpha), cx);

  PbrMRMaterialUniforms::default()
    .with_source(base_color)
    .with_source(emissive)
    .with_source(normal_mapping_scale)
    .with_source(roughness)
    .with_source(metallic)
    .with_source(alpha)
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, ShaderStruct, Debug, PartialEq, Default)]
pub struct PhysicalMetallicRoughnessMaterialTextureHandlesUniform {
  pub base_color_texture: TextureSamplerHandlePair,
  pub emissive_texture: TextureSamplerHandlePair,
  pub metallic_roughness_texture: TextureSamplerHandlePair,
  pub normal_texture: TextureSamplerHandlePair,
}
type TexUniform = PhysicalMetallicRoughnessMaterialTextureHandlesUniform;

pub type PbrMRMaterialTexUniforms = UniformUpdateContainer<PbrMRMaterialEntity, TexUniform>;
pub fn pbr_mr_material_tex_uniforms(cx: &GPUResourceCtx) -> PbrMRMaterialTexUniforms {
  let tex_offset = offset_of!(TextureSamplerHandlePair, texture_handle);
  let sam_offset = offset_of!(TextureSamplerHandlePair, sampler_handle);

  let base_color_texture = global_watch()
    .watch_typed_key::<SceneTexture2dRefOf<PbrMRMaterialBaseColorTex>>()
    .collective_map(|id| id.unwrap_or(0))
    .into_uniform_collection_update(offset_of!(TexUniform, base_color_texture) + tex_offset, cx);

  let base_color_sampler = global_watch()
    .watch_typed_key::<SceneSamplerRefOf<PbrMRMaterialBaseColorTex>>()
    .collective_map(|id| id.unwrap_or(0))
    .into_uniform_collection_update(offset_of!(TexUniform, base_color_texture) + sam_offset, cx);

  PbrMRMaterialTexUniforms::default()
    .with_source(base_color_texture)
    .with_source(base_color_sampler)
}
