use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct DirectionalLightStorage {
  /// in lx
  pub illuminance: Vec3<f32>,
  pub direction: Vec3<f32>,
}

pub fn use_directional_light_storage(
  qcx: &mut QueryGPUHookCx,
) -> Option<LightGPUStorage<DirectionalLightStorage>> {
  let (qcx, light) = qcx.use_storage_buffer2(128, u32::MAX);

  qcx
    .use_changes::<DirectionalLightIlluminance>()
    .update_storage_array(light, offset_of!(DirectionalLightStorage, illuminance));

  let fanout = global_node_world_mat(qcx)
    .fanout(qcx.use_db_rev_ref_tri_view::<DirectionalRefNode>())
    .into_delta_change()
    .map(|change| change.collective_map(|mat| mat.forward().reverse().normalize().into_f32()));

  qcx
    .use_result(fanout)
    .update_storage_array(light, offset_of!(PointLightStorage, position));

  let multi_access = qcx.use_gpu_general_query(|gpu| {
    MultiAccessGPUDataBuilder::new(
      gpu,
      global_rev_ref().watch_inv_ref_untyped::<DirectionalRefScene>(),
      light_multi_access_config(),
    )
  });
  qcx.when_render(|| {
    let light = light.get_gpu_buffer();
    let multi_access = multi_access.unwrap();
    (light, multi_access)
  })
}

pub fn make_dir_light_storage_component(
  (light_data, light_accessor): &LightGPUStorage<DirectionalLightStorage>,
) -> Box<dyn LightingComputeComponent> {
  Box::new(AllInstanceOfGivenTypeLightInScene::new(
    light_accessor.clone(),
    light_data.clone(),
    |light| {
      let light = light.load().expand();
      ENode::<DirectionalShaderInfo> {
        illuminance: light.illuminance,
        direction: light.direction,
      }
      .construct()
    },
  ))
}
