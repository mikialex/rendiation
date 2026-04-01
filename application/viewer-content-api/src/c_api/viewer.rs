use crate::*;

#[no_mangle]
pub extern "C" fn create_viewer_content_api_instance() -> *mut ViewerAPI {
  let init_config = ViewerInitConfig::default();
  let api = ViewerAPI::new(init_config);
  let api = Box::new(api);
  Box::leak(api)
}

#[no_mangle]
pub extern "C" fn drop_viewer_content_api_instance(api: *mut ViewerAPI) {
  let _ = unsafe { Box::from_raw(api) };
}

/// hinstance can be null_ptr
#[no_mangle]
pub extern "C" fn viewer_create_surface(
  api: &mut ViewerAPI,
  hwnd: *mut c_void,
  hinstance: *mut c_void,
  width: u32,
  height: u32,
) -> u32 {
  api.create_surface(hwnd, hinstance, width, height)
}

#[no_mangle]
pub extern "C" fn viewer_drop_surface(api: *mut ViewerAPI, surface_id: u32) {
  let api = unsafe { &mut *api };
  api.drop_surface(surface_id)
}

/// may return empty handle for error case
#[no_mangle]
pub extern "C" fn viewer_read_last_render_result(
  api: *mut ViewerAPI,
  surface_id: u32,
) -> ViewerEntityHandle {
  let api = unsafe { &mut *api };

  if let Some(data) = api.read_last_render_result(surface_id) {
    let data = MaybeUriData::Living(Arc::new(data));
    let data = ExternalRefPtr::new(data);
    global_entity_of::<SceneTexture2dEntity>()
      .entity_writer()
      .new_entity(|w| w.write::<SceneTexture2dEntityDirectContent>(&Some(data)))
      .into()
  } else {
    ViewerEntityHandle::empty()
  }
}

/// the size is physical resolution
#[no_mangle]
pub extern "C" fn viewer_resize(
  api: *mut ViewerAPI,
  surface_id: u32,
  new_width: u32,
  new_height: u32,
) {
  let api = unsafe { &mut *api };
  api.resize(surface_id, new_width, new_height);
}

#[no_mangle]
pub extern "C" fn viewer_render_surface(api: &mut ViewerAPI, surface_id: u32) {
  api.render_surface(surface_id);
}

#[no_mangle]
pub extern "C" fn viewer_create_picker_api(
  api: &mut ViewerAPI,
  surface_id: u32,
) -> *mut ViewerPickerAPI {
  let api = api.create_picker_api(surface_id);
  let api = Box::new(api);
  Box::leak(api)
}

/// picker api must be dropped before any scene related modifications, or deadlock will occur
#[no_mangle]
pub extern "C" fn viewer_drop_picker_api(api: *mut ViewerPickerAPI) {
  let _ = unsafe { Box::from_raw(api) };
}

/// the returned pick list's should be dropped by  [drop_pick_list_result] after read the result
#[no_mangle]
pub extern "C" fn picker_pick_list(
  api: *mut ViewerPickerAPI,
  viewer: *mut ViewerAPI,
  scene: ViewerEntityHandle,
  x: f32,
  y: f32,
) -> *mut ViewerRayPickListResult {
  let api = unsafe { &mut *api };
  let viewer = unsafe { &mut *viewer };
  let mut pick_results = Vec::new();
  api.pick_list(&viewer.viewer, scene.into(), x, y, &mut pick_results);

  let r = Box::new(ViewerRayPickListResult { pick_results });
  Box::leak(r)
}

#[no_mangle]
pub extern "C" fn drop_pick_list_result(r: *mut ViewerRayPickListResult) {
  unsafe {
    let _ = Box::from_raw(r);
  };
}

pub struct ViewerRayPickListResult {
  pick_results: Vec<ViewerRayPickResult>,
}

#[repr(C)]
pub struct ViewerRayPickListResultInfo {
  pub len: usize,
  pub ptr: *const ViewerRayPickResult,
}

#[no_mangle]
pub extern "C" fn get_ray_pick_list_info(
  r: *mut ViewerRayPickListResult,
) -> ViewerRayPickListResultInfo {
  let r = unsafe { &*r };
  ViewerRayPickListResultInfo {
    len: r.pick_results.len(),
    ptr: r.pick_results.as_ptr(),
  }
}

#[no_mangle]
pub extern "C" fn create_scene() -> ViewerEntityHandle {
  global_entity_of::<SceneEntity>()
    .entity_writer()
    .new_entity(|w| w)
    .into()
}
#[no_mangle]
pub extern "C" fn drop_scene(handle: ViewerEntityHandle) {
  global_entity_of::<SceneEntity>()
    .entity_writer()
    .delete_entity(handle.into());
}
