use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct DirectionalLightStorage {
  /// in lx
  pub illuminance: Vec3<f32>,
  pub direction: Vec3<f32>,
}

pub fn directional_storage(gpu: &GPU) -> ReactiveStorageBufferContainer<DirectionalLightStorage> {
  let illuminance_offset = offset_of!(DirectionalLightStorage, illuminance);
  let illuminance = global_watch()
    .watch::<DirectionalLightIlluminance>()
    .into_query_update_storage(illuminance_offset);

  let direction = scene_node_derive_world_mat()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<DirectionalRefNode>())
    .collective_map(|mat| mat.forward().reverse().normalize().into_f32())
    .into_query_update_storage(offset_of!(DirectionalLightStorage, direction));

  create_reactive_storage_buffer_container(128, u32::MAX, gpu)
    .with_source(illuminance)
    .with_source(direction)
}

pub fn use_directional_light_storage(
  qcx: &mut impl QueryGPUHookCx,
) -> Option<LightGPUStorage<DirectionalLightStorage>> {
  let light = qcx.use_storage_buffer(directional_storage);
  let multi_access = qcx.use_gpu_general_query(|gpu| {
    MultiAccessGPUDataBuilder::new(
      gpu,
      global_rev_ref().watch_inv_ref_untyped::<DirectionalRefScene>(),
      light_multi_access_config(),
    )
  });
  qcx.when_render(|| {
    let light = light.unwrap();
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
