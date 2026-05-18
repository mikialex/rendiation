use crate::*;

/// the content format expects Rgba8UnormSrgb
#[no_mangle]
pub extern "C" fn create_texture2d(
  content: *const u8,
  len: usize,
  width: u32,
  height: u32,
  format: TextureFormat,
) -> ViewerEntityHandle {
  let data = unsafe { slice::from_raw_parts(content, len) };
  let data = data.to_vec();
  let data = GPUBufferImage {
    data,
    format,
    size: Size::from_u32_pair_min_one((width, height)),
  };
  let data = MaybeUriData::Living(Arc::new(data));
  let data = ExternalRefPtr::new(data);
  global_entity_of::<SceneTexture2dEntity>()
    .entity_writer()
    .new_entity(|w| w.write::<SceneTexture2dEntityDirectContent>(&Some(data)))
    .into()
}

#[no_mangle]
pub extern "C" fn update_texture2d_content(
  handle: ViewerEntityHandle,
  content: *const u8,
  len: usize,
  width: u32,
  height: u32,
  format: wgpu_types::TextureFormat,
) {
  let data = unsafe { slice::from_raw_parts(content, len) };
  let data = data.to_vec();
  let data = GPUBufferImage {
    data,
    format,
    size: Size::from_u32_pair_min_one((width, height)),
  };
  let data = MaybeUriData::Living(Arc::new(data));
  let data = ExternalRefPtr::new(data);
  write_global_db_component::<SceneTexture2dEntityDirectContent>().write(handle.into(), Some(data));
}

#[no_mangle]
pub extern "C" fn create_texture_cube() -> ViewerEntityHandle {
  global_entity_of::<SceneTextureCubeEntity>()
    .entity_writer()
    .new_entity(|w| w)
    .into()
}

#[no_mangle]
pub extern "C" fn drop_texture_cube(handle: ViewerEntityHandle) {
  global_entity_of::<SceneTextureCubeEntity>()
    .entity_writer()
    .delete_entity(handle.into())
}

#[no_mangle]
pub extern "C" fn texture_cube_set_face(
  cube: ViewerEntityHandle,
  face_index: u32,
  tex: ViewerEntityHandle,
) {
  let cube: EntityHandle<SceneTextureCubeEntity> = cube.into();
  let tex: EntityHandle<SceneTexture2dEntity> = tex.into();
  let tex = tex.some_handle();
  match face_index {
    0 => write_global_db_component::<SceneTextureCubeXPositiveFace>().write(cube, tex),
    1 => write_global_db_component::<SceneTextureCubeYPositiveFace>().write(cube, tex),
    2 => write_global_db_component::<SceneTextureCubeZPositiveFace>().write(cube, tex),
    3 => write_global_db_component::<SceneTextureCubeXNegativeFace>().write(cube, tex),
    4 => write_global_db_component::<SceneTextureCubeYNegativeFace>().write(cube, tex),
    5 => write_global_db_component::<SceneTextureCubeZNegativeFace>().write(cube, tex),
    _ => {
      log::warn!("texture_cube_set_face: invalid face_index {face_index}, expected 0-5");
      false
    }
  };
}

#[no_mangle]
pub extern "C" fn drop_texture2d(handle: ViewerEntityHandle) {
  global_entity_of::<SceneTexture2dEntity>()
    .entity_writer()
    .delete_entity(handle.into());
}

#[no_mangle]
pub extern "C" fn create_sampler() -> ViewerEntityHandle {
  global_entity_of::<SceneSamplerEntity>()
    .entity_writer()
    .new_entity(|w| w)
    .into()
}
#[no_mangle]
pub extern "C" fn drop_sampler(handle: ViewerEntityHandle) {
  global_entity_of::<SceneSamplerEntity>()
    .entity_writer()
    .delete_entity(handle.into());
}
