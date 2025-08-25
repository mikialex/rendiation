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
  cx: &mut QueryGPUHookCx,
) -> Option<LightGPUStorage<DirectionalLightStorage>> {
  let (cx, light) = cx.use_storage_buffer("directional lights", 128, u32::MAX);

  cx.use_changes::<DirectionalLightIlluminance>()
    .update_storage_array(light, offset_of!(DirectionalLightStorage, illuminance));

  use_global_node_world_mat(cx)
    .fanout(cx.use_db_rev_ref_tri_view::<DirectionalRefNode>())
    .into_delta_change()
    .map(|change| change.collective_map(|mat| mat.forward().reverse().normalize().into_f32()))
    .use_assure_result(cx)
    .update_storage_array(light, offset_of!(DirectionalLightStorage, direction));

  let (cx, multi_acc) = cx.use_gpu_multi_access_states(light_multi_access_config());

  let updates = cx
    .use_db_rev_ref_tri_view::<DirectionalRefScene>()
    .use_assure_result(cx);

  cx.when_render(|| {
    let light = light.get_gpu_buffer();
    let multi_access = multi_acc.update(updates.expect_resolve_stage());
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
