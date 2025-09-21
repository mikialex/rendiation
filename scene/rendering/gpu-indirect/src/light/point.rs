use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct PointLightStorage {
  /// in cd
  pub luminance_intensity: Vec3<f32>,
  pub position: HighPrecisionTranslationStorage,
  pub cutoff_distance: f32,
}

pub fn use_point_light_storage(
  cx: &mut QueryGPUHookCx,
) -> Option<LightGPUStorage<PointLightStorage>> {
  let (cx, light) = cx.use_storage_buffer("point lights", 128, u32::MAX);

  let offset = offset_of!(PointLightStorage, luminance_intensity);
  cx.use_changes::<PointLightIntensity>()
    .update_storage_array(cx, light, offset);

  cx.use_changes::<PointLightCutOffDistance>()
    .update_storage_array(cx, light, offset_of!(PointLightStorage, cutoff_distance));

  use_global_node_world_mat(cx)
    .fanout(cx.use_db_rev_ref_tri_view::<PointLightRefNode>(), cx)
    .into_delta_change()
    .map(|change| change.collective_map(|mat| into_hpt(mat.position()).into_storage()))
    .update_storage_array(cx, light, offset_of!(PointLightStorage, position));

  light.use_max_item_count_by_db_entity::<PointLightEntity>(cx);
  light.use_update(cx);

  let updates = cx.use_db_rev_ref_tri_view::<PointLightRefScene>();
  let multi_access = use_multi_access_gpu(cx, &light_multi_access_config(), updates, "point light");

  cx.when_render(|| {
    let light = light.get_gpu_buffer();
    (light, multi_access.unwrap())
  })
}

pub fn make_point_light_storage_component(
  (light_data, light_accessor): &LightGPUStorage<PointLightStorage>,
) -> Box<dyn LightingComputeComponent> {
  Box::new(AllInstanceOfGivenTypeLightInScene::new(
    light_accessor.clone(),
    light_data.clone(),
    |light| {
      let light = light.load().expand();
      ENode::<PointLightShaderInfo> {
        luminance_intensity: light.luminance_intensity,
        position: hpt_storage_to_hpt(light.position),
        cutoff_distance: light.cutoff_distance,
      }
      .construct()
    },
  ))
}
