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

#[no_mangle]
pub extern "C" fn create_viewer_content_api_instance() -> *mut ViewerAPI {
  todo!()
}
#[no_mangle]
pub extern "C" fn viewer_resize(api: *mut ViewerAPI, new_width: u32, new_height: u32) {
  let api = unsafe { &mut *api };
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
