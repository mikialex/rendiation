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
  handle: SceneModelHandleInfo,
  scene: *const ViewerEntityHandle,
) {
  if scene.is_null() {
    write_global_db_component::<SceneModelBelongsToScene>().write(handle.scene_model.into(), None);
  } else {
    write_global_db_component::<SceneModelBelongsToScene>()
      .write(handle.scene_model.into(), Some(unsafe { *scene }.into()));
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
pub extern "C" fn scene_model_set_z_layer(handle: ViewerEntityHandle, z_layer: OccStyleZLayer) {
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
pub extern "C" fn drop_wide_points(p: SceneWidePointsHandleInfo) {
  global_entity_of::<WideStyledPointsEntity>()
    .entity_writer()
    .delete_entity(p.points.into());

  global_entity_of::<SceneModelEntity>()
    .entity_writer()
    .delete_entity(p.scene_model.into());
}
