use crate::*;

#[no_mangle]
pub extern "C" fn create_node() -> ViewerEntityHandle {
  global_entity_of::<SceneNodeEntity>()
    .entity_writer()
    .new_entity(|w| w)
    .into()
}

#[no_mangle]
pub extern "C" fn delete_node(node: ViewerEntityHandle) {
  global_entity_of::<SceneNodeEntity>()
    .entity_writer()
    .delete_entity(node.into());
}

#[no_mangle]
pub extern "C" fn node_set_local_mat(node: ViewerEntityHandle, mat4: *const [f64; 16]) {
  let mat4 = unsafe { *mat4 };
  let mat4 = Mat4::from(mat4);
  let mut writer = global_entity_component_of::<SceneNodeLocalMatrixComponent, _>(|c| c.write());
  writer.write(node.into(), mat4);
}

// #[no_mangle]
// pub extern "C" fn node_get_world_mat(node: ViewerEntityHandle, mat4: *const [f64; 16]) {
//   let mat4 = unsafe { *mat4 };
//   todo!();
// }

/// set parent to null_ptr to detach
#[no_mangle]
pub extern "C" fn node_attach_parent(node: ViewerEntityHandle, parent: *mut ViewerEntityHandle) {
  let mut writer = global_entity_component_of::<SceneNodeParentIdx, _>(|c| c.write());

  if parent.is_null() {
    writer.write(node.into(), None);
    return;
  } else {
    let parent = unsafe { *parent };
    writer.write(node.into(), Some(parent.into()));
  }
}
