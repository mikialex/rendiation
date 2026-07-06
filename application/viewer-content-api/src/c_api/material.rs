use crate::*;

#[no_mangle]
pub extern "C" fn create_occ_material() -> ViewerEntityHandle {
  global_entity_of::<OccStyleMaterialEntity>()
    .entity_writer()
    .new_entity(|w| w)
    .into()
}

#[no_mangle]
pub extern "C" fn drop_occ_material(handle: ViewerEntityHandle) {
  global_entity_of::<OccStyleMaterialEntity>()
    .entity_writer()
    .delete_entity(handle.into())
}

#[no_mangle]
pub extern "C" fn occ_material_set_diffuse(mat: ViewerEntityHandle, color: &[f32; 4]) {
  write_global_db_component::<OccStyleMaterialDiffuse>().write(mat.into(), (*color).into());
}

#[no_mangle]
pub extern "C" fn occ_material_set_specular(mat: ViewerEntityHandle, color: &[f32; 3]) {
  write_global_db_component::<OccStyleMaterialSpecular>().write(mat.into(), (*color).into());
}

#[no_mangle]
pub extern "C" fn occ_material_set_shininess(mat: ViewerEntityHandle, shininess: f32) {
  write_global_db_component::<OccStyleMaterialShininess>().write(mat.into(), shininess);
}

#[no_mangle]
pub extern "C" fn occ_material_set_emissive(mat: ViewerEntityHandle, color: &[f32; 3]) {
  write_global_db_component::<OccStyleMaterialEmissive>().write(mat.into(), (*color).into());
}

#[no_mangle]
pub extern "C" fn create_occ_effect_control() -> ViewerEntityHandle {
  global_entity_of::<OccStyleEffectControlEntity>()
    .entity_writer()
    .new_entity(|w| w)
    .into()
}

#[no_mangle]
pub extern "C" fn drop_occ_effect_control(handle: ViewerEntityHandle) {
  global_entity_of::<OccStyleEffectControlEntity>()
    .entity_writer()
    .delete_entity(handle.into())
}

#[no_mangle]
pub extern "C" fn occ_material_set_effect(mat: ViewerEntityHandle, effect: ViewerEntityHandle) {
  write_global_db_component::<OccStyleMaterialEffect>().write(mat.into(), Some(effect.into()));
}

#[no_mangle]
pub extern "C" fn occ_effect_control_set_shade_type(
  effect: ViewerEntityHandle,
  shade_type: OccStyleEffectType,
) {
  write_global_db_component::<OccStyleEffectShadeType>().write(effect.into(), shade_type);
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum CullMode {
  None,
  Front,
  Back,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OccControlStateSimple {
  enable_depth_test: bool,
  enable_depth_write: bool,
  front_face_ccw: bool,
  depth_bias_constant_factor: f32,
  depth_bias_slop_factor: f32,
  depth_bias_clamp: f32,
  enable_alpha_blend: bool,
  cull_mode: CullMode,
}

#[no_mangle]
pub extern "C" fn occ_effect_control_set_state(
  effect: ViewerEntityHandle,
  simple_config: OccControlStateSimple,
) {
  let state = RasterizationStates {
    depth_compare: if simple_config.enable_depth_test {
      SemanticCompareFunction::Nearer
    } else {
      SemanticCompareFunction::Always
    },
    depth_write_enabled: simple_config.enable_depth_write,
    front_face: if simple_config.front_face_ccw {
      FrontFace::Ccw
    } else {
      FrontFace::Cw
    },
    bias: DepthBiasState {
      constant: simple_config.depth_bias_constant_factor as i32,
      slope_scale: simple_config.depth_bias_slop_factor,
      clamp: simple_config.depth_bias_clamp,
    },
    blend: if simple_config.enable_alpha_blend {
      Some(BlendState::ALPHA_BLENDING)
    } else {
      None
    },
    cull_mode: match simple_config.cull_mode {
      CullMode::None => None,
      CullMode::Front => Some(Face::Front),
      CullMode::Back => Some(Face::Back),
    },
    ..Default::default()
  };
  write_global_db_component::<OccStyleEffectStateOverride>().write(effect.into(), Some(state));
}

#[no_mangle]
pub extern "C" fn occ_material_set_diffuse_tex(
  mat: ViewerEntityHandle,
  tex: ViewerEntityHandle,
  sampler: ViewerEntityHandle,
) {
  write_tex_sampler::<OccStyleMaterialDiffuseTex>(mat, tex, sampler)
}

#[no_mangle]
pub extern "C" fn std_model_set_occ_material(
  handle: ViewerEntityHandle,
  material: ViewerEntityHandle,
) {
  write_global_db_component::<StdModelOccStyleMaterialPayload>()
    .write(handle.into(), Some(material.into()));
}

#[no_mangle]
pub extern "C" fn create_unlit_material() -> ViewerEntityHandle {
  global_entity_of::<UnlitMaterialEntity>()
    .entity_writer()
    .new_entity(|w| w)
    .into()
}
#[no_mangle]
pub extern "C" fn unlit_material_set_color(mat: ViewerEntityHandle, color: &[f32; 4]) {
  write_global_db_component::<UnlitMaterialColorComponent>().write(mat.into(), (*color).into());
}

#[no_mangle]
pub extern "C" fn drop_unlit_material(handle: ViewerEntityHandle) {
  global_entity_of::<UnlitMaterialEntity>()
    .entity_writer()
    .delete_entity(handle.into())
}

#[no_mangle]
pub extern "C" fn create_pbr_mr_material() -> ViewerEntityHandle {
  global_entity_of::<PbrMRMaterialEntity>()
    .entity_writer()
    .new_entity(|w| w)
    .into()
}

#[no_mangle]
pub extern "C" fn pbr_mr_material_set_base_color(mat: ViewerEntityHandle, color: &[f32; 3]) {
  write_global_db_component::<PbrMRMaterialBaseColorComponent>().write(mat.into(), (*color).into());
}
#[no_mangle]
pub extern "C" fn pbr_mr_material_set_base_color_tex(
  mat: ViewerEntityHandle,
  tex: ViewerEntityHandle,
  sampler: ViewerEntityHandle,
) {
  write_tex_sampler::<PbrMRMaterialBaseColorAlphaTex>(mat, tex, sampler)
}

pub(crate) fn write_tex_sampler<C: TextureWithSamplingForeignKeys>(
  target: ViewerEntityHandle,
  tex: ViewerEntityHandle,
  sampler: ViewerEntityHandle,
) {
  write_global_db_component::<SceneTexture2dRefOf<C>>().write(target.into(), Some(tex.into()));
  write_global_db_component::<SceneSamplerRefOf<C>>().write(target.into(), Some(sampler.into()));
}

#[no_mangle]
pub extern "C" fn drop_pbr_mr_material(handle: ViewerEntityHandle) {
  global_entity_of::<PbrMRMaterialEntity>()
    .entity_writer()
    .delete_entity(handle.into())
}
