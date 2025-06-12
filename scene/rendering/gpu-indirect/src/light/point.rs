use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct PointLightStorage {
  /// in cd
  pub luminance_intensity: Vec3<f32>,
  pub position: Vec3<f32>,
  pub cutoff_distance: f32,
}

pub fn point_storage(gpu: &GPU) -> ReactiveStorageBufferContainer<PointLightStorage> {
  let luminance_intensity_offset = offset_of!(PointLightStorage, luminance_intensity);
  let luminance_intensity = global_watch()
    .watch::<PointLightIntensity>()
    .into_query_update_storage(luminance_intensity_offset);

  let cutoff_distance_offset = offset_of!(PointLightStorage, cutoff_distance);
  let cutoff_distance = global_watch()
    .watch::<PointLightCutOffDistance>()
    .into_query_update_storage(cutoff_distance_offset);

  let position = scene_node_derive_world_mat()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<PointLightRefNode>())
    .collective_map(|mat| mat.position())
    .into_query_update_storage(offset_of!(PointLightStorage, position));

  create_reactive_storage_buffer_container(128, u32::MAX, gpu)
    .with_source(luminance_intensity)
    .with_source(cutoff_distance)
    .with_source(position)
}

pub fn use_point_light_storage(
  qcx: &mut impl QueryGPUHookCx,
) -> Option<LightGPUStorage<PointLightStorage>> {
  let light = qcx.use_storage_buffer(point_storage);
  let multi_access = qcx.use_gpu_general_query(|gpu| {
    MultiAccessGPUDataBuilder::new(
      gpu,
      global_rev_ref().watch_inv_ref_untyped::<PointLightRefScene>(),
      light_multi_access_config(),
    )
  });
  qcx.when_render(|| {
    let light = light.unwrap();
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
        position: light.position,
        cutoff_distance: light.cutoff_distance,
      }
      .construct()
    },
  ))
}
