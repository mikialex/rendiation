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
  qcx: &mut QueryGPUHookCx,
) -> Option<LightGPUStorage<PointLightStorage>> {
  let (qcx, light) = qcx.use_storage_buffer2(128, u32::MAX);

  qcx
    .use_changes::<PointLightIntensity>()
    .update_storage_array(light, offset_of!(PointLightStorage, luminance_intensity));

  qcx
    .use_changes::<PointLightCutOffDistance>()
    .update_storage_array(light, offset_of!(PointLightStorage, cutoff_distance));

  use_global_node_world_mat(qcx)
    .fanout(qcx.use_db_rev_ref_tri_view::<PointLightRefNode>())
    .into_delta_change()
    .map(|change| change.collective_map(|mat| into_hpt(mat.position()).into_storage()))
    .use_assure_result(qcx)
    .update_storage_array(light, offset_of!(PointLightStorage, position));

  let (qcx, multi_acc) = qcx.use_gpu_multi_access_states(light_multi_access_config());

  let updates = qcx
    .use_db_rev_ref_tri_view::<PointLightRefScene>()
    .use_assure_result(qcx);

  qcx.when_render(|| {
    let light = light.get_gpu_buffer();
    let multi_access = multi_acc.update(updates.expect_resolve_stage());
    (light, multi_access)
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
