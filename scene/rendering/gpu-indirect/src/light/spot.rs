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
  qcx: &mut QueryGPUHookCx,
) -> Option<LightGPUStorage<SpotLightStorage>> {
  let (qcx, light) = qcx.use_storage_buffer2(128, u32::MAX);

  qcx
    .use_changes::<SpotLightIntensity>()
    .update_storage_array(light, offset_of!(SpotLightStorage, luminance_intensity));

  qcx
    .use_changes::<SpotLightCutOffDistance>()
    .update_storage_array(light, offset_of!(SpotLightStorage, cutoff_distance));

  qcx
    .use_changes::<SpotLightHalfConeAngle>()
    .map_changes(|rad| rad.cos())
    .update_storage_array(light, offset_of!(SpotLightStorage, half_cone_cos));

  qcx
    .use_changes::<SpotLightHalfPenumbraAngle>()
    .map_changes(|rad| rad.cos())
    .update_storage_array(light, offset_of!(SpotLightStorage, half_penumbra_cos));

  let fanout = use_global_node_world_mat(qcx)
    .fanout(qcx.use_db_rev_ref_tri_view::<SpotLightRefNode>())
    .use_assure_result(qcx);

  fanout
    .clone_except_future()
    .into_delta_change()
    .map(|change| change.collective_map(|mat| into_hpt(mat.position()).into_storage()))
    .update_storage_array(light, offset_of!(SpotLightStorage, position));

  fanout
    .into_delta_change()
    .map(|change| change.collective_map(|mat| mat.forward().reverse().normalize().into_f32()))
    .update_storage_array(light, offset_of!(SpotLightStorage, direction));

  let (qcx, multi_acc) = qcx.use_gpu_multi_access_states(light_multi_access_config());

  let updates = qcx
    .use_db_rev_ref_tri_view::<SpotLightRefScene>()
    .use_assure_result(qcx);

  qcx.when_render(|| {
    let light = light.get_gpu_buffer();
    let multi_access = multi_acc.update(updates.expect_resolve_stage());
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
