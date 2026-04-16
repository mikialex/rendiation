use crate::*;

#[no_mangle]
pub extern "C" fn create_dir_light(node: ViewerEntityHandle) -> ViewerEntityHandle {
  global_entity_of::<DirectionalLightEntity>()
    .entity_writer()
    .new_entity(|w| w.write::<DirectionalRefNode>(&Some(node.into())))
    .into()
}

#[no_mangle]
pub extern "C" fn set_dir_light_scene(
  handle: ViewerEntityHandle,
  scene: *const ViewerEntityHandle,
) {
  if scene.is_null() {
    write_global_db_component::<DirectionalRefScene>().write(handle.into(), None);
  } else {
    write_global_db_component::<DirectionalRefScene>()
      .write(handle.into(), Some(unsafe { *scene }.into()));
  }
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

#[no_mangle]
pub extern "C" fn create_point_light(node: ViewerEntityHandle) -> ViewerEntityHandle {
  global_entity_of::<PointLightEntity>()
    .entity_writer()
    .new_entity(|w| w.write::<PointLightRefNode>(&Some(node.into())))
    .into()
}

#[no_mangle]
pub extern "C" fn set_point_light_scene(
  handle: ViewerEntityHandle,
  scene: *const ViewerEntityHandle,
) {
  if scene.is_null() {
    write_global_db_component::<PointLightRefScene>().write(handle.into(), None);
  } else {
    write_global_db_component::<PointLightRefScene>()
      .write(handle.into(), Some(unsafe { *scene }.into()));
  }
}

#[no_mangle]
pub extern "C" fn set_point_light_intensity(node: ViewerEntityHandle, illuminance: &[f32; 3]) {
  write_global_db_component::<PointLightIntensity>().write(node.into(), (*illuminance).into());
}
#[no_mangle]
pub extern "C" fn set_point_light_cutoff_distance(node: ViewerEntityHandle, distance: f32) {
  write_global_db_component::<PointLightCutOffDistance>().write(node.into(), distance);
}

#[no_mangle]
pub extern "C" fn drop_point_light(handle: ViewerEntityHandle) {
  global_entity_of::<PointLightEntity>()
    .entity_writer()
    .delete_entity(handle.into());
}

#[no_mangle]
pub extern "C" fn create_spot_light(node: ViewerEntityHandle) -> ViewerEntityHandle {
  global_entity_of::<SpotLightEntity>()
    .entity_writer()
    .new_entity(|w| w.write::<SpotLightRefNode>(&Some(node.into())))
    .into()
}

#[no_mangle]
pub extern "C" fn set_spot_light_scene(
  handle: ViewerEntityHandle,
  scene: *const ViewerEntityHandle,
) {
  if scene.is_null() {
    write_global_db_component::<SpotLightRefScene>().write(handle.into(), None);
  } else {
    write_global_db_component::<SpotLightRefScene>()
      .write(handle.into(), Some(unsafe { *scene }.into()));
  }
}

#[no_mangle]
pub extern "C" fn set_spot_light_intensity(node: ViewerEntityHandle, illuminance: &[f32; 3]) {
  write_global_db_component::<SpotLightIntensity>().write(node.into(), (*illuminance).into());
}

#[no_mangle]
pub extern "C" fn set_spot_light_cutoff_distance(node: ViewerEntityHandle, distance: f32) {
  write_global_db_component::<SpotLightCutOffDistance>().write(node.into(), distance);
}

#[no_mangle]
pub extern "C" fn set_spot_light_half_cone_angle(node: ViewerEntityHandle, angle: f32) {
  write_global_db_component::<SpotLightHalfConeAngle>().write(node.into(), angle);
}

#[no_mangle]
pub extern "C" fn set_spot_light_half_penumbra_angle(node: ViewerEntityHandle, angle: f32) {
  write_global_db_component::<SpotLightHalfPenumbraAngle>().write(node.into(), angle);
}

#[no_mangle]
pub extern "C" fn drop_spot_light(handle: ViewerEntityHandle) {
  global_entity_of::<SpotLightEntity>()
    .entity_writer()
    .delete_entity(handle.into());
}
