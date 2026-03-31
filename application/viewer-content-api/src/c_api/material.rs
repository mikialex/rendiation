use crate::*;

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
pub extern "C" fn pbr_mr_material_set_color(mat: ViewerEntityHandle, color: &[f32; 3]) {
  write_global_db_component::<PbrMRMaterialBaseColorComponent>().write(mat.into(), (*color).into());
}
#[no_mangle]
pub extern "C" fn pbr_mr_material_set_color_tex(
  mat: ViewerEntityHandle,
  tex: ViewerEntityHandle,
  sampler: ViewerEntityHandle,
) {
  write_tex_sampler::<PbrMRMaterialBaseColorAlphaTex>(mat, tex, sampler)
}

fn write_tex_sampler<C: TextureWithSamplingForeignKeys>(
  mat: ViewerEntityHandle,
  tex: ViewerEntityHandle,
  sampler: ViewerEntityHandle,
) {
  write_global_db_component::<SceneTexture2dRefOf<C>>().write(mat.into(), Some(tex.into()));
  write_global_db_component::<SceneSamplerRefOf<C>>().write(mat.into(), Some(sampler.into()));
}

#[no_mangle]
pub extern "C" fn drop_pbr_mr_material(handle: ViewerEntityHandle) {
  global_entity_of::<PbrMRMaterialEntity>()
    .entity_writer()
    .delete_entity(handle.into())
}
