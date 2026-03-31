use crate::*;

/// the content format expects Rgba8UnormSrgb
#[no_mangle]
pub extern "C" fn create_texture2d(
  content: *const u8,
  len: usize,
  width: u32,
  height: u32,
) -> ViewerEntityHandle {
  let data = unsafe { slice::from_raw_parts(content, len) };
  let data = data.to_vec();
  let data = GPUBufferImage {
    data,
    format: raw_gpu::TextureFormat::Rgba8UnormSrgb,
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
