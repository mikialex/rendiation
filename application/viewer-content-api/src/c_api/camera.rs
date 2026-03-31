use crate::*;

#[no_mangle]
pub extern "C" fn create_camera(node: ViewerEntityHandle) -> ViewerEntityHandle {
  global_entity_of::<SceneCameraEntity>()
    .entity_writer()
    .new_entity(|w| w.write::<SceneCameraNode>(&Some(node.into())))
    .into()
}
#[no_mangle]
pub extern "C" fn drop_camera(handle: ViewerEntityHandle) {
  global_entity_of::<SceneCameraEntity>()
    .entity_writer()
    .delete_entity(handle.into());
}

#[no_mangle]
pub extern "C" fn camera_set_proj_perspective(
  handle: ViewerEntityHandle,
  near: f32,
  far: f32,
  vertical_fov_in_deg: f32,
  aspect: f32,
) {
  let handle = handle.into();
  write_global_db_component::<SceneCameraOrthographic>().write(handle, None);
  write_global_db_component::<SceneCameraPerspective>().write(
    handle,
    PerspectiveProjection {
      near,
      far,
      fov: Deg::by(vertical_fov_in_deg),
      aspect,
    }
    .into(),
  );
}

#[no_mangle]
pub extern "C" fn camera_set_proj_orth(
  handle: ViewerEntityHandle,
  near: f32,
  far: f32,
  left: f32,
  right: f32,
  top: f32,
  bottom: f32,
) {
  let handle = handle.into();
  write_global_db_component::<SceneCameraPerspective>().write(handle, None);
  write_global_db_component::<SceneCameraOrthographic>().write(
    handle,
    OrthographicProjection {
      near,
      far,
      left,
      right,
      top,
      bottom,
    }
    .into(),
  );
}
