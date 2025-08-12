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

  let node_world_mat = global_node_derive_of::<SceneNodeLocalMatrixComponent, _>(node_world_mat);
  let node_world_mat = qcx.use_shared_compute(node_world_mat);

  let fanout = node_world_mat
    .fanout(qcx.use_db_rev_ref_tri_view::<PointLightRefNode>())
    .map(|change| {
      change
        .delta
        .into_change()
        .collective_map(|mat| into_hpt(mat.position()).into_storage())
    });

  qcx
    .use_result(fanout)
    .update_storage_array(light, offset_of!(PointLightStorage, position));

  let multi_access = qcx.use_gpu_general_query(|gpu| {
    MultiAccessGPUDataBuilder::new(
      gpu,
      global_rev_ref().watch_inv_ref_untyped::<PointLightRefScene>(),
      light_multi_access_config(),
    )
  });
  qcx.when_render(|| {
    let light = light.get_gpu_buffer();
    let multi_access = multi_access.unwrap();
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
