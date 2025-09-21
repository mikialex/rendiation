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
    .update_storage_array(cx, light, offset_of!(DirectionalLightStorage, illuminance));

  use_global_node_world_mat(cx)
    .fanout(cx.use_db_rev_ref_tri_view::<DirectionalRefNode>(), cx)
    .into_delta_change()
    .map(|change| change.collective_map(|mat| mat.forward().reverse().normalize().into_f32()))
    .update_storage_array(cx, light, offset_of!(DirectionalLightStorage, direction));

  let updates = cx.use_db_rev_ref_tri_view::<DirectionalRefScene>();
  let multi_access = use_multi_access_gpu(
    cx,
    &light_multi_access_config(),
    updates,
    "directional light",
  );

  light.use_max_item_count_by_db_entity::<DirectionalLightEntity>(cx);
  light.use_update(cx);

  cx.when_render(|| {
    let light = light.get_gpu_buffer();
    (light, multi_access.unwrap())
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
