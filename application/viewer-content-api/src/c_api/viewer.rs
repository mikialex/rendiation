use std::{
  ffi::{c_char, CStr},
  path::Path,
};

use crate::*;

#[no_mangle]
pub extern "C" fn create_viewer_content_api_instance(config_path: *const c_char) -> *mut ViewerAPI {
  let config_path = unsafe { CStr::from_ptr(config_path) };
  let init_config = if let Ok(config_path) = config_path.to_str() {
    if let Some(r) = ViewerInitConfig::from_toml_or_default(config_path) {
      r
    } else {
      log::warn!("unable to read or parse the config file, use default config");
      ViewerInitConfig::default()
    }
  } else {
    log::warn!("unable to convert c style config path into utf8, use default config");
    ViewerInitConfig::default()
  };

  log::info!("create viewer api instance");

  let api = ViewerAPI::new(init_config);
  let api = Box::new(api);
  Box::leak(api)
}

#[no_mangle]
pub extern "C" fn drop_viewer_content_api_instance(api: *mut ViewerAPI) {
  let _ = unsafe { Box::from_raw(api) };
}

#[no_mangle]
pub extern "C" fn viewer_set_tonemap_ty_value(
  api: &mut ViewerAPI,
  ty: rendiation_texture_gpu_process::ToneMapType,
  exposure: f32,
) {
  api.core.viewer.rendering.lighting.tonemap.ty = ty;
  api
    .core
    .viewer
    .rendering
    .lighting
    .tonemap
    .set_exposure(exposure);
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
pub extern "C" fn viewer_drop_surface(api: &mut ViewerAPI, surface_id: u32) {
  api.drop_surface(surface_id)
}

#[no_mangle]
pub extern "C" fn viewer_surface_set_camera(
  api: &mut ViewerAPI,
  surface_id: u32,
  camera: ViewerEntityHandle,
) {
  api.set_surface_camera(surface_id, camera.into());
}

#[no_mangle]
pub extern "C" fn viewer_surface_set_scene(
  api: &mut ViewerAPI,
  surface_id: u32,
  scene: ViewerEntityHandle,
) {
  api.set_surface_scene(surface_id, scene.into());
}

#[no_mangle]
pub extern "C" fn viewer_set_enable_clip(
  api: &mut ViewerAPI,
  enable_clip: bool,
  enable_clip_fill: bool,
) {
  api.core.viewer.rendering.use_array_clip = enable_clip;
  api.core.viewer.rendering.fill_clip_face = enable_clip_fill;
}

/// may return empty handle for error case
#[no_mangle]
pub extern "C" fn viewer_read_last_render_result(
  api: &mut ViewerAPI,
  surface_id: u32,
) -> ViewerEntityHandle {
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
  api: &mut ViewerAPI,
  surface_id: u32,
  new_width: u32,
  new_height: u32,
) {
  api.resize(surface_id, new_width, new_height);
}

#[no_mangle]
pub extern "C" fn viewer_load_font(api: &mut ViewerAPI, font_path: *const c_char) {
  let font_path = unsafe { CStr::from_ptr(font_path) };
  if let Ok(s) = font_path.to_str() {
    let font_path = Path::new(s);

    match std::fs::read(&font_path) {
      Ok(data) => api.core.viewer.load_font(data),
      Err(e) => log::error!("failed to read font from {:?}, error: {e:?}", font_path),
    }
  } else {
    log::error!("invalid font path: {font_path:?}");
  }
}

#[no_mangle]
pub extern "C" fn viewer_render_surface(api: &mut ViewerAPI, surface_id: u32) {
  api.render_surface(surface_id);
}

#[no_mangle]
pub extern "C" fn viewer_create_world_derive_query_api(
  api: &mut ViewerAPI,
) -> *mut ViewerWorldDeriveQueryAPI {
  let api = api.create_world_derive_query_api();
  let api = Box::new(api);
  Box::leak(api)
}

/// api must be dropped before any scene related modifications, or deadlock will occur
#[no_mangle]
pub extern "C" fn viewer_drop_world_derive_query_api(api: *mut ViewerWorldDeriveQueryAPI) {
  let _ = unsafe { Box::from_raw(api) };
}

#[no_mangle]
pub extern "C" fn world_derive_query_api_get_world_mat(
  api: &mut ViewerWorldDeriveQueryAPI,
  node: ViewerEntityHandle,
  r: &mut [f64; 16],
) -> bool {
  if let Some(mat) = api.world_mats.access(&node.into()) {
    *r = mat.into();
    true
  } else {
    false
  }
}

#[no_mangle]
pub extern "C" fn world_derive_query_api_get_world_bounding(
  api: &mut ViewerWorldDeriveQueryAPI,
  sm: ViewerEntityHandle,
  result: &mut [f64; 6],
) -> bool {
  if let Some(Some(bbox)) = api.sm_world_bound.access(&sm.into()) {
    result[0] = bbox.min.x;
    result[1] = bbox.min.y;
    result[2] = bbox.min.z;
    result[3] = bbox.max.x;
    result[4] = bbox.max.y;
    result[5] = bbox.max.z;

    true
  } else {
    false
  }
}

#[no_mangle]
pub extern "C" fn world_derive_query_api_get_local_bounding(
  api: &mut ViewerWorldDeriveQueryAPI,
  sm: ViewerEntityHandle,
  result: &mut [f32; 6],
) -> bool {
  if let Some(bbox) = api.sm_local_bound.access(&sm.into()) {
    result[0] = bbox.min.x;
    result[1] = bbox.min.y;
    result[2] = bbox.min.z;
    result[3] = bbox.max.x;
    result[4] = bbox.max.y;
    result[5] = bbox.max.z;

    true
  } else {
    false
  }
}

#[no_mangle]
pub extern "C" fn viewer_create_picker_api(
  api: &mut ViewerAPI,
  surface_id: u32,
) -> *mut ViewerQueryAPI {
  let api = api.create_query_api(surface_id);
  let api = Box::new(api);
  Box::leak(api)
}

/// api must be dropped before any scene related modifications, or deadlock will occur
#[no_mangle]
pub extern "C" fn viewer_drop_picker_api(api: *mut ViewerQueryAPI) {
  let _ = unsafe { Box::from_raw(api) };
}

#[no_mangle]
pub extern "C" fn query_scene_bounding(
  api: &mut ViewerWorldDeriveQueryAPI,
  viewer_api: &mut ViewerAPI,
  scene: ViewerEntityHandle,
  result: &mut [f32; 6],
  consider_override: bool,
  surface_id: u32,
) {
  let active_view = if consider_override {
    let surface_content = viewer_api
      .core
      .viewer
      .surfaces_content
      .get(&surface_id)
      .unwrap();
    Some(surface_content.viewports[0].id)
  } else {
    None
  };

  let bbox = api
    .scene_bounding
    .get_or_compute_scene_bounding(scene.into(), active_view);

  result[0] = bbox.min.x;
  result[1] = bbox.min.y;
  result[2] = bbox.min.z;

  result[3] = bbox.max.x;
  result[4] = bbox.max.y;
  result[5] = bbox.max.z;
}

/// the returned pick list's should be dropped by  [drop_pick_list_result] after read the result
///
/// all inputs are logic pixel
#[no_mangle]
pub extern "C" fn picker_pick_list(
  api: &mut ViewerQueryAPI,
  viewer: &mut ViewerAPI,
  x: f32,
  y: f32,
  extra_screen_space_tolerance: f32,
  sort_near_to_far: bool,
) -> *mut ViewerRayPickListResult {
  let mut pick_results = Vec::new();
  api.pick_list(
    &viewer.core.viewer,
    x,
    y,
    extra_screen_space_tolerance,
    &mut pick_results,
  );

  let camera_position_world = api.get_camera_position_world(&viewer.core.viewer);

  if sort_near_to_far {
    let camera_position_world = camera_position_world.into_f32();
    pick_results.sort_by_cached_key(|a| {
      let distance_sq = Vec3::from(a.hit_position).distance2_to(camera_position_world);
      ordered_float::OrderedFloat::from(distance_sq)
    });
  }

  let r = Box::new(ViewerRayPickListResult {
    pick_results,
    camera_position_world,
  });
  Box::leak(r)
}

#[no_mangle]
pub extern "C" fn drop_pick_list_result(r: *mut ViewerRayPickListResult) {
  unsafe {
    let _ = Box::from_raw(r);
  };
}

/// the returned pick range's should be dropped by  [drop_pick_range_result] after read the result
///
/// the a, b point can be swapped without order limits.
///
/// all inputs are logic pixel
#[no_mangle]
pub extern "C" fn picker_pick_range(
  api: &mut ViewerQueryAPI,
  viewer: &mut ViewerAPI,
  ax: f32,
  ay: f32,
  bx: f32,
  by: f32,
  contains: bool,
  precise_intersection_test: bool,
  extra_screen_space_tolerance: f32,
) -> *mut ViewerRayPickRangeResult {
  let mut pick_results = Vec::new();
  api.pick_range(
    &viewer.core.viewer,
    ax,
    ay,
    bx,
    by,
    &mut pick_results,
    contains,
    precise_intersection_test,
    extra_screen_space_tolerance,
  );

  let r = Box::new(ViewerRayPickRangeResult { pick_results });
  Box::leak(r)
}

#[no_mangle]
pub extern "C" fn drop_pick_range_result(r: *mut ViewerRayPickRangeResult) {
  unsafe {
    let _ = Box::from_raw(r);
  };
}

pub struct ViewerRayPickRangeResult {
  pick_results: Vec<ViewerEntityHandle>,
}
#[repr(C)]
pub struct ViewerRayPickRangeResultInfo {
  pub len: usize,
  pub ptr: *const ViewerEntityHandle,
}

#[no_mangle]
pub extern "C" fn get_ray_pick_range_info(
  r: *mut ViewerRayPickRangeResult,
) -> ViewerRayPickRangeResultInfo {
  let r = unsafe { &*r };
  ViewerRayPickRangeResultInfo {
    len: r.pick_results.len(),
    ptr: r.pick_results.as_ptr(),
  }
}

pub struct ViewerRayPickListResult {
  pick_results: Vec<ViewerRayPickResult>,
  camera_position_world: Vec3<f64>,
}

#[repr(C)]
pub struct ViewerRayPickListResultInfo {
  pub len: usize,
  pub ptr: *const ViewerRayPickResult,
  pub camera_position_world: [f64; 3],
}

#[no_mangle]
pub extern "C" fn get_ray_pick_list_info(
  r: *mut ViewerRayPickListResult,
) -> ViewerRayPickListResultInfo {
  let r = unsafe { &*r };
  ViewerRayPickListResultInfo {
    len: r.pick_results.len(),
    ptr: r.pick_results.as_ptr(),
    camera_position_world: r.camera_position_world.into(),
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

#[no_mangle]
pub extern "C" fn scene_set_background_solid(handle: ViewerEntityHandle, color: &[f32; 3]) {
  write_global_db_component::<SceneSolidBackground>().write(handle.into(), Some((*color).into()));
  write_global_db_component::<SceneGradientBackgroundInfo>().write(handle.into(), None);
}

#[no_mangle]
pub extern "C" fn scene_set_background_gradient(
  handle: ViewerEntityHandle,
  top: &[f32; 3],
  bottom: &[f32; 3],
) {
  let top: Vec3<f32> = (*top).into();
  let bottom: Vec3<f32> = (*bottom).into();
  write_global_db_component::<SceneGradientBackgroundInfo>().write(
    handle.into(),
    Some(SceneGradientBackgroundParam {
      transform: Mat4::identity(),
      color_and_stops: vec![top.expand_with(0.), bottom.expand_with(1.)],
    }),
  );
  write_global_db_component::<SceneSolidBackground>().write(handle.into(), None);
}
