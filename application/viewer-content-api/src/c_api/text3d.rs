use crate::*;

#[repr(C)]
#[derive(Default)]
pub struct Text3dQueryInfoC {
  pub min_x: f32,
  pub min_y: f32,
  pub max_x: f32,
  pub max_y: f32,
  pub cap_a_height: f32,
  pub units_per_em: u32,
  pub has_result: bool,
}

#[unsafe(no_mangle)]
pub extern "C" fn text3d_query(
  api: &mut ViewerAPI,
  handle: ViewerEntityHandle,
) -> Text3dQueryInfoC {
  let mut rr = Text3dQueryInfoC::default();
  let mut font_sys = api.core.viewer.font_system.write();
  if let Some(r) = compute_text_layout_info(handle.into(), &mut font_sys) {
    let bbox = r.local_bbox;
    rr.min_x = bbox.min.x;
    rr.min_y = bbox.min.y;
    rr.max_x = bbox.max.x;
    rr.max_y = bbox.max.y;
    rr.units_per_em = r.units_per_em;
    rr.cap_a_height = r.cap_a_height;
    rr.has_result = true;
  } else {
    rr.has_result = false;
  }
  rr
}
