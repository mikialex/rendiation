use std::ffi::{c_char, CStr};

use crate::*;

#[repr(C)]
pub struct SceneModelHandleInfo {
  scene_model: ViewerEntityHandle,
  std_model: ViewerEntityHandle,
}

#[no_mangle]
pub extern "C" fn create_scene_model(
  material: ViewerEntityHandle,
  is_unlit_material: bool, // or pbr mr
  mesh: ViewerEntityHandle,
  node: ViewerEntityHandle,
  scene: ViewerEntityHandle,
) -> SceneModelHandleInfo {
  let std_model = global_entity_of::<StandardModelEntity>()
    .entity_writer()
    .new_entity(|w| {
      let w = w.write::<StandardModelRefAttributesMeshEntity>(&Some(mesh.into()));
      if is_unlit_material {
        w.write::<StandardModelRefUnlitMaterial>(&Some(material.into()))
      } else {
        w.write::<StandardModelRefPbrMRMaterial>(&Some(material.into()))
      }
    });

  let scene_model = global_entity_of::<SceneModelEntity>()
    .entity_writer()
    .new_entity(|w| {
      w.write::<SceneModelBelongsToScene>(&Some(scene.into()))
        .write::<SceneModelRefNode>(&Some(node.into()))
        .write::<SceneModelStdModelRenderPayload>(&Some(std_model.into_raw()))
    });

  SceneModelHandleInfo {
    std_model: std_model.into(),
    scene_model: scene_model.into(),
  }
}

#[no_mangle]
pub extern "C" fn drop_scene_model(handle: SceneModelHandleInfo) {
  global_entity_of::<StandardModelEntity>()
    .entity_writer()
    .delete_entity(handle.std_model.into());

  global_entity_of::<SceneModelEntity>()
    .entity_writer()
    .delete_entity(handle.scene_model.into());
}

#[no_mangle]
pub extern "C" fn scene_model_set_mesh(handle: SceneModelHandleInfo, mesh: ViewerEntityHandle) {
  write_global_db_component::<StandardModelRefAttributesMeshEntity>()
    .write(handle.std_model.into(), Some(mesh.into()));
}

#[no_mangle]
pub extern "C" fn scene_model_set_scene(
  handle: ViewerEntityHandle,
  scene: *const ViewerEntityHandle,
) {
  if scene.is_null() {
    write_global_db_component::<SceneModelBelongsToScene>().write(handle.into(), None);
  } else {
    write_global_db_component::<SceneModelBelongsToScene>()
      .write(handle.into(), Some(unsafe { *scene }.into()));
  }
}

#[no_mangle]
pub extern "C" fn scene_model_set_occ_style_view_dep(
  handle: ViewerEntityHandle,
  is_2d: bool,
  anchor: &[f32; 3],
  offset: &[i32; 2],
  corner: u32,
  mode: u32,
) {
  let transform_ty = if is_2d {
    OccStyleTransform::Dimension2 {
      offset: (*offset).into(),
      corner: OccStyleCorner::from_bits_retain(corner),
    }
  } else {
    OccStyleTransform::Dimension3 {
      anchor_point: (*anchor).into(),
    }
  };

  let config = OccStyleViewDepConfig {
    transform_ty,
    mode: OccStyleMode::from_bits_retain(mode),
  };
  write_global_db_component::<SceneModelViewDependentTransformOcc>()
    .write(handle.into(), Some(config));
}

#[no_mangle]
pub extern "C" fn scene_model_remove_occ_style_view_dep(handle: ViewerEntityHandle) {
  write_global_db_component::<SceneModelViewDependentTransformOcc>().write(handle.into(), None);
}

#[no_mangle]
pub extern "C" fn scene_model_set_z_layer(handle: ViewerEntityHandle, z_layer: OccFlavorZLayer) {
  write_global_db_component::<SceneModelOccStyleLayer>().write(handle.into(), z_layer);
}

#[no_mangle]
pub extern "C" fn scene_model_set_priority(handle: ViewerEntityHandle, priority: u32) {
  write_global_db_component::<SceneModelOccStylePriority>().write(handle.into(), priority);
}

#[no_mangle]
pub extern "C" fn scene_model_set_selectable(handle: ViewerEntityHandle, selectable: bool) {
  write_global_db_component::<SceneModelSelectable>().write(handle.into(), selectable);
}

#[no_mangle]
pub extern "C" fn scene_model_set_material(
  handle: SceneModelHandleInfo,
  material: ViewerEntityHandle,
  is_unlit_material: bool,
) {
  if is_unlit_material {
    write_global_db_component::<StandardModelRefUnlitMaterial>()
      .write(handle.std_model.into(), Some(material.into()));
  } else {
    write_global_db_component::<StandardModelRefUnlitMaterial>()
      .write(handle.std_model.into(), Some(material.into()));
  }
}

#[repr(C)]
pub struct SceneWidePointsHandleInfo {
  scene_model: ViewerEntityHandle,
  points: ViewerEntityHandle,
}

#[no_mangle]
pub extern "C" fn create_wide_points(
  node: ViewerEntityHandle,
  data_length: u32,
  data: *const u8,
) -> SceneWidePointsHandleInfo {
  let mut writer = global_entity_of::<WideStyledPointsEntity>().entity_writer();

  let data = unsafe { slice::from_raw_parts(data, data_length as usize) };
  let data = data.to_vec();
  let data = ExternalRefPtr::new(data);

  let points = writer.new_entity(|w| w.write::<WideStyledPointsMeshBuffer>(&data));

  let scene_model = global_entity_of::<SceneModelEntity>()
    .entity_writer()
    .new_entity(|w| {
      w.write::<SceneModelWideStyledPointsRenderPayload>(&points.some_handle())
        .write::<SceneModelRefNode>(&Some(node.into()))
    });

  SceneWidePointsHandleInfo {
    scene_model: scene_model.into(),
    points: points.into(),
  }
}

#[no_mangle]
pub extern "C" fn wide_points_set_buffer(
  handle: ViewerEntityHandle,
  data_length: u32,
  data: *const u8,
) {
  let data = unsafe { slice::from_raw_parts(data, data_length as usize) };
  let data = data.to_vec();
  let data = ExternalRefPtr::new(data);

  write_global_db_component::<WideStyledPointsMeshBuffer>().write(handle.into(), data.into());
}

#[no_mangle]
pub extern "C" fn wide_points_set_color(handle: ViewerEntityHandle, color: &[f32; 4]) {
  write_global_db_component::<WideStyledPointsColor>().write(handle.into(), (*color).into());
}

#[no_mangle]
pub extern "C" fn wide_points_set_pattern_texture(
  handle: ViewerEntityHandle,
  texture: ViewerEntityHandle,
  sampler: ViewerEntityHandle,
) {
  write_tex_sampler::<WidePointsColorAlphaTex>(handle, texture, sampler)
}

#[no_mangle]
pub extern "C" fn drop_wide_points(p: SceneWidePointsHandleInfo) {
  global_entity_of::<WideStyledPointsEntity>()
    .entity_writer()
    .delete_entity(p.points.into());

  global_entity_of::<SceneModelEntity>()
    .entity_writer()
    .delete_entity(p.scene_model.into());
}

#[repr(C)]
pub struct SceneWideLineHandleInfo {
  scene_model: ViewerEntityHandle,
  line: ViewerEntityHandle,
}

#[no_mangle]
pub extern "C" fn create_wide_line(
  node: ViewerEntityHandle,
  data_length: u32,
  data: *const u8,
) -> SceneWideLineHandleInfo {
  let mut writer = global_entity_of::<WideLineModelEntity>().entity_writer();

  let data = unsafe { slice::from_raw_parts(data, data_length as usize) };
  let data = data.to_vec();
  let data = ExternalRefPtr::new(data);

  let line = writer.new_entity(|w| w.write::<WideLineMeshBuffer>(&data));

  let scene_model = global_entity_of::<SceneModelEntity>()
    .entity_writer()
    .new_entity(|w| {
      w.write::<SceneModelWideLineRenderPayload>(&line.some_handle())
        .write::<SceneModelRefNode>(&Some(node.into()))
    });

  SceneWideLineHandleInfo {
    scene_model: scene_model.into(),
    line: line.into(),
  }
}

#[no_mangle]
pub extern "C" fn wide_line_set_buffer(
  handle: ViewerEntityHandle,
  data_length: u32,
  data: *const u8,
) {
  let data = unsafe { slice::from_raw_parts(data, data_length as usize) };
  let data = data.to_vec();
  let data = ExternalRefPtr::new(data);

  write_global_db_component::<WideLineMeshBuffer>().write(handle.into(), data.into());
}

#[no_mangle]
pub extern "C" fn wide_line_set_color(handle: ViewerEntityHandle, color: &[f32; 4]) {
  write_global_db_component::<WideLineColor>().write(handle.into(), (*color).into());
}

#[no_mangle]
pub extern "C" fn wide_line_set_width(handle: ViewerEntityHandle, width: &f32) {
  write_global_db_component::<WideLineWidth>().write(handle.into(), *width);
}

#[no_mangle]
pub extern "C" fn wide_line_set_pattern(handle: ViewerEntityHandle, pattern: u32) {
  write_global_db_component::<WideLineStylePattern>().write(handle.into(), pattern);
}

#[no_mangle]
pub extern "C" fn wide_line_set_factor(handle: ViewerEntityHandle, factor: f32) {
  write_global_db_component::<WideLineStyleFactor>().write(handle.into(), factor);
}

#[no_mangle]
pub extern "C" fn drop_wide_line(p: SceneWideLineHandleInfo) {
  global_entity_of::<WideLineModelEntity>()
    .entity_writer()
    .delete_entity(p.line.into());

  global_entity_of::<SceneModelEntity>()
    .entity_writer()
    .delete_entity(p.scene_model.into());
}

#[repr(C)]
pub struct Text3dContentInfoC {
  pub content: *const c_char,
  pub font_size: f32,
  pub line_height: f32,
  pub scale: f32,
  pub font: *const c_char,
  pub weight: u32,
  pub has_weight: bool,
  pub color: [f32; 4],
  pub italic: bool,
  pub width: f32,
  pub has_width: bool,
  pub height: f32,
  pub has_height: bool,
  pub align: TextAlignment,
}

#[repr(C)]
pub struct SceneText3dHandleInfo {
  scene_model: ViewerEntityHandle,
  text3d: ViewerEntityHandle,
}

#[no_mangle]
pub extern "C" fn create_text3d(
  node: ViewerEntityHandle,
  content: *const Text3dContentInfoC,
) -> SceneText3dHandleInfo {
  let mut writer = global_entity_of::<Text3dEntity>().entity_writer();
  let content = text3d_content_from_c(content);

  let text3d = writer.new_entity(|w| w.write::<Text3dContent>(&content));

  let scene_model = global_entity_of::<SceneModelEntity>()
    .entity_writer()
    .new_entity(|w| {
      w.write::<SceneModelText3dPayload>(&text3d.some_handle())
        .write::<SceneModelRefNode>(&Some(node.into()))
    });

  SceneText3dHandleInfo {
    scene_model: scene_model.into(),
    text3d: text3d.into(),
  }
}

#[no_mangle]
pub extern "C" fn text3d_set_content(
  handle: ViewerEntityHandle,
  content: *const Text3dContentInfoC,
) {
  let content = text3d_content_from_c(content);
  write_global_db_component::<Text3dContent>().write(handle.into(), content);
}

fn parse_optional_c_string(ptr: *const c_char) -> Option<String> {
  if ptr.is_null() {
    None
  } else {
    Some(
      unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned(),
    )
  }
}

fn text3d_content_from_c(
  info: *const Text3dContentInfoC,
) -> Option<ExternalRefPtr<Text3dContentInfo>> {
  let info = unsafe { info.as_ref() }?;

  Some(ExternalRefPtr::new(Text3dContentInfo {
    content: parse_optional_c_string(info.content).unwrap_or_default(),
    font_size: info.font_size,
    line_height: info.line_height,
    scale: info.scale,
    font: parse_optional_c_string(info.font),
    weight: info.has_weight.then_some(info.weight),
    color: info.color.into(),
    italic: info.italic,
    width: info.has_width.then_some(info.width),
    height: info.has_height.then_some(info.height),
    align: info.align.into(),
  }))
}

#[no_mangle]
pub extern "C" fn drop_text3d(p: SceneText3dHandleInfo) {
  global_entity_of::<Text3dEntity>()
    .entity_writer()
    .delete_entity(p.text3d.into());

  global_entity_of::<SceneModelEntity>()
    .entity_writer()
    .delete_entity(p.scene_model.into());
}
