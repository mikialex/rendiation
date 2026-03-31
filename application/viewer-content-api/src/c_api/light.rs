use crate::*;

#[no_mangle]
pub extern "C" fn create_dir_light(node: ViewerEntityHandle) -> ViewerEntityHandle {
  global_entity_of::<DirectionalLightEntity>()
    .entity_writer()
    .new_entity(|w| w.write::<DirectionalRefNode>(&Some(node.into())))
    .into()
}

#[no_mangle]
pub extern "C" fn set_dir_light_illuminance(node: ViewerEntityHandle, illuminance: &[f32; 3]) {
  write_global_db_component::<DirectionalLightIlluminance>()
    .write(node.into(), (*illuminance).into());
}

#[no_mangle]
pub extern "C" fn drop_dir_light(handle: ViewerEntityHandle) {
  global_entity_of::<DirectionalLightEntity>()
    .entity_writer()
    .delete_entity(handle.into());
}
