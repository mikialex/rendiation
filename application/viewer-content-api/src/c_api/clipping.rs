use crate::*;

#[no_mangle]
pub extern "C" fn create_clipping_plane(
  plane: &[f32; 4],
  scene: *const ViewerEntityHandle,
) -> ViewerEntityHandle {
  global_entity_of::<ClippingPlaneEntity>()
    .entity_writer()
    .new_entity(|w| {
      let w = w.write::<ClippingPlaneInfo>(&(*plane).into());
      if !scene.is_null() {
        w.write::<ClippingPlaneRefScene>(&Some(unsafe { *scene }.into()))
      } else {
        w
      }
    })
    .into()
}

#[no_mangle]
pub extern "C" fn drop_clipping_plane(handle: ViewerEntityHandle) {
  global_entity_of::<ClippingPlaneEntity>()
    .entity_writer()
    .delete_entity(handle.into());
}

#[no_mangle]
pub extern "C" fn clipping_plane_set_plane(handle: ViewerEntityHandle, plane: &[f32; 4]) {
  write_global_db_component::<ClippingPlaneInfo>().write(handle.into(), (*plane).into());
}

#[no_mangle]
pub extern "C" fn clipping_plane_set_scene(
  handle: ViewerEntityHandle,
  scene: *const ViewerEntityHandle,
) {
  if scene.is_null() {
    write_global_db_component::<ClippingPlaneRefScene>().write(handle.into(), None);
  } else {
    write_global_db_component::<ClippingPlaneRefScene>()
      .write(handle.into(), Some(unsafe { *scene }.into()));
  }
}

#[no_mangle]
pub extern "C" fn attribute_mesh_set_is_solid(handle: ViewerEntityHandle, is_solid: bool) {
  write_global_db_component::<AttributeMeshIsSolid>().write(handle.into(), is_solid);
}
