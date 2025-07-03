use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct SpotLightUniform {
  pub luminance_intensity: Vec3<f32>,
  pub position: HighPrecisionTranslationUniform,
  pub direction: Vec3<f32>,
  pub cutoff_distance: f32,
  pub half_cone_cos: f32,
  pub half_penumbra_cos: f32,
}

pub fn spot_uniform_array(gpu: &GPU) -> UniformArrayUpdateContainer<SpotLightUniform, 8> {
  let buffer = UniformBufferDataView::create_default(&gpu.device);

  let luminance_intensity = global_watch()
    .watch::<SplitLightIntensity>()
    .into_query_update_uniform_array(offset_of!(SpotLightUniform, luminance_intensity), gpu);

  let cutoff_distance = global_watch()
    .watch::<SpotLightCutOffDistance>()
    .into_query_update_uniform_array(offset_of!(SpotLightUniform, cutoff_distance), gpu);

  let half_cone_cos = global_watch()
    .watch::<SpotLightHalfConeAngle>()
    .collective_map(|rad| rad.cos())
    .into_query_update_uniform_array(offset_of!(SpotLightUniform, half_cone_cos), gpu);

  let half_penumbra_cos = global_watch()
    .watch::<SpotLightHalfPenumbraAngle>()
    .collective_map(|rad| rad.cos())
    .into_query_update_uniform_array(offset_of!(SpotLightUniform, half_penumbra_cos), gpu);

  let world = scene_node_derive_world_mat()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SpotLightRefNode>())
    .into_forker();

  let position = world
    .clone()
    .collective_map(|mat| into_hpt(mat.position()).into_uniform())
    .into_query_update_uniform_array(offset_of!(SpotLightUniform, position), gpu);

  let direction = world
    .collective_map(|mat| mat.forward().reverse().normalize().into_f32())
    .into_query_update_uniform_array(offset_of!(SpotLightUniform, direction), gpu);

  UniformArrayUpdateContainer::new(buffer)
    .with_source(luminance_intensity)
    .with_source(cutoff_distance)
    .with_source(half_cone_cos)
    .with_source(half_penumbra_cos)
    .with_source(position)
    .with_source(direction)
}
