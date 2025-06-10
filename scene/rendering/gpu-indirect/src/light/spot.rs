use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct SpotLightStorage {
  pub luminance_intensity: Vec3<f32>,
  pub position: Vec3<f32>,
  pub direction: Vec3<f32>,
  pub cutoff_distance: f32,
  pub half_cone_cos: f32,
  pub half_penumbra_cos: f32,
}

pub fn spot_storage(gpu: &GPU) -> ReactiveStorageBufferContainer<SpotLightStorage> {
  let luminance_intensity = global_watch()
    .watch::<SplitLightIntensity>()
    .into_query_update_storage(offset_of!(SpotLightStorage, luminance_intensity));

  let cutoff_distance = global_watch()
    .watch::<SpotLightCutOffDistance>()
    .into_query_update_storage(offset_of!(SpotLightStorage, cutoff_distance));

  let half_cone_cos = global_watch()
    .watch::<SpotLightHalfConeAngle>()
    .collective_map(|rad| rad.cos())
    .into_query_update_storage(offset_of!(SpotLightStorage, half_cone_cos));

  let half_penumbra_cos = global_watch()
    .watch::<SpotLightHalfPenumbraAngle>()
    .collective_map(|rad| rad.cos())
    .into_query_update_storage(offset_of!(SpotLightStorage, half_penumbra_cos));

  let world = scene_node_derive_world_mat()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SpotLightRefNode>())
    .into_forker();

  let position = world
    .clone()
    .collective_map(|mat| mat.position())
    .into_query_update_storage(offset_of!(SpotLightStorage, position));

  let direction = world
    .collective_map(|mat| mat.forward().reverse().normalize())
    .into_query_update_storage(offset_of!(SpotLightStorage, direction));

  create_reactive_storage_buffer_container(128, u32::MAX, gpu)
    .with_source(luminance_intensity)
    .with_source(cutoff_distance)
    .with_source(half_cone_cos)
    .with_source(half_penumbra_cos)
    .with_source(position)
    .with_source(direction)
}

pub fn use_spot_light_storage(
  qcx: &mut impl QueryGPUHookCx,
) -> Option<LightGPUStorage<SpotLightStorage>> {
  let light = qcx.use_storage_buffer(spot_storage);
  let multi_access = qcx.use_gpu_general_query(|gpu| {
    MultiAccessGPUDataBuilder::new(
      gpu,
      global_rev_ref().watch_inv_ref_untyped::<SpotLightRefScene>(),
      light_multi_access_config(),
    )
  });
  qcx.when_render(|| {
    let light = light.unwrap();
    let multi_access = multi_access.unwrap();
    (light, multi_access)
  })
}

pub fn make_spot_light_storage_component(
  (light_data, light_accessor): &LightGPUStorage<SpotLightStorage>,
) -> Box<dyn LightingComputeComponent> {
  Box::new(AllInstanceOfGivenTypeLightInScene::new(
    light_accessor.clone(),
    light_data.clone(),
    |light| {
      let light = light.load().expand();
      ENode::<SpotLightShaderInfo> {
        luminance_intensity: light.luminance_intensity,
        position: light.position,
        direction: light.direction,
        cutoff_distance: light.cutoff_distance,
        half_cone_cos: light.half_cone_cos,
        half_penumbra_cos: light.half_penumbra_cos,
      }
      .construct()
    },
  ))
}
