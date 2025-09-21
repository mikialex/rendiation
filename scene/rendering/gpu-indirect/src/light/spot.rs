use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct SpotLightStorage {
  pub luminance_intensity: Vec3<f32>,
  pub position: HighPrecisionTranslationStorage,
  pub direction: Vec3<f32>,
  pub cutoff_distance: f32,
  pub half_cone_cos: f32,
  pub half_penumbra_cos: f32,
}

pub fn use_spot_light_storage(
  cx: &mut QueryGPUHookCx,
) -> Option<LightGPUStorage<SpotLightStorage>> {
  let (cx, light) = cx.use_storage_buffer("spot lights", 128, u32::MAX);

  let offset = offset_of!(SpotLightStorage, luminance_intensity);
  cx.use_changes::<SpotLightIntensity>()
    .update_storage_array(cx, light, offset);

  cx.use_changes::<SpotLightCutOffDistance>()
    .update_storage_array(cx, light, offset_of!(SpotLightStorage, cutoff_distance));

  cx.use_changes::<SpotLightHalfConeAngle>()
    .map_changes(|rad| rad.cos())
    .update_storage_array(cx, light, offset_of!(SpotLightStorage, half_cone_cos));

  cx.use_changes::<SpotLightHalfPenumbraAngle>()
    .map_changes(|rad| rad.cos())
    .update_storage_array(cx, light, offset_of!(SpotLightStorage, half_penumbra_cos));

  let (fanout, fanout_) = use_global_node_world_mat(cx)
    .fanout(cx.use_db_rev_ref_tri_view::<SpotLightRefNode>(), cx)
    .fork();

  fanout
    .into_delta_change()
    .map(|change| change.collective_map(|mat| into_hpt(mat.position()).into_storage()))
    .update_storage_array(cx, light, offset_of!(SpotLightStorage, position));

  fanout_
    .into_delta_change()
    .map(|change| change.collective_map(|mat| mat.forward().reverse().normalize().into_f32()))
    .update_storage_array(cx, light, offset_of!(SpotLightStorage, direction));

  light.use_max_item_count_by_db_entity::<SpotLightEntity>(cx);
  light.use_update(cx);

  let updates = cx.use_db_rev_ref_tri_view::<SpotLightRefScene>();
  let multi_access = use_multi_access_gpu(cx, &light_multi_access_config(), updates, "spot light");

  cx.when_render(|| {
    let light = light.get_gpu_buffer();
    (light, multi_access.unwrap())
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
        position: hpt_storage_to_hpt(light.position),
        direction: light.direction,
        cutoff_distance: light.cutoff_distance,
        half_cone_cos: light.half_cone_cos,
        half_penumbra_cos: light.half_penumbra_cos,
      }
      .construct()
    },
  ))
}
