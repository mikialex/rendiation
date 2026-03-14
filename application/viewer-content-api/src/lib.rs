use rendiation_viewer_content::*;

pub struct ViewerAPI {
  gpu_and_surface: WGPUAndSurface,
  viewer: Viewer,
}

impl ViewerAPI {
  pub fn render(&mut self) {
    //
  }
}

#[repr(C)]
pub struct ViewerEntityHandle {
  pub index: u32,
  pub generation: u64,
}

#[no_mangle]
pub extern "C" fn create_viewer_content_api_instance(hwnd: i32) -> *mut ViewerAPI {
  todo!()
}
#[no_mangle]
pub extern "C" fn viewer_resize(api: *mut ViewerAPI, new_width: u32, new_height: u32) {
  let api = unsafe { &mut *api };
  todo!()
}

#[no_mangle]
pub extern "C" fn viewer_create_node(api: *mut ViewerAPI) -> ViewerEntityHandle {
  todo!()
}
#[no_mangle]
pub extern "C" fn viewer_delete_node(api: *mut ViewerAPI, node: ViewerEntityHandle) {
  todo!()
}

#[no_mangle]
pub extern "C" fn viewer_render(api: *mut ViewerAPI) {
  let api = unsafe { &mut *api };
  api.render();
}

#[no_mangle]
pub extern "C" fn drop_viewer_content_api_instance(api: *mut ViewerAPI) {
  let _ = unsafe { Box::from_raw(api) };
}
