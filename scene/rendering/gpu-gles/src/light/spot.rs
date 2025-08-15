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

pub fn use_spot_uniform_array(cx: &mut QueryGPUHookCx) -> UniformArray<SpotLightUniform, 8> {
  let (cx, uniform) = cx.use_uniform_array_buffers();

  let offset = offset_of!(SpotLightUniform, luminance_intensity);
  cx.use_changes::<PointLightIntensity>()
    .update_uniform_array(uniform, offset, cx.gpu);

  let offset = offset_of!(SpotLightUniform, cutoff_distance);
  cx.use_changes::<PointLightCutOffDistance>()
    .update_uniform_array(uniform, offset, cx.gpu);

  let offset = offset_of!(SpotLightUniform, half_cone_cos);
  cx.use_changes::<SpotLightHalfConeAngle>()
    .map_changes(|rad| rad.cos())
    .update_uniform_array(uniform, offset, cx.gpu);

  let offset = offset_of!(SpotLightUniform, half_penumbra_cos);
  cx.use_changes::<SpotLightHalfPenumbraAngle>()
    .map_changes(|rad| rad.cos())
    .update_uniform_array(uniform, offset, cx.gpu);

  let fanout = use_global_node_world_mat(cx)
    .fanout(cx.use_db_rev_ref_tri_view::<SpotLightRefNode>())
    .use_assure_result(cx);

  fanout
    .clone_except_future()
    .into_delta_change()
    .map(|change| change.collective_map(|mat| into_hpt(mat.position()).into_storage()))
    .update_uniform_array(uniform, offset_of!(SpotLightUniform, position), cx.gpu);

  fanout
    .into_delta_change()
    .map(|change| change.collective_map(|mat| mat.forward().reverse().normalize().into_f32()))
    .update_uniform_array(uniform, offset_of!(SpotLightUniform, direction), cx.gpu);

  uniform.clone()
}
