use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct PointLightUniform {
  /// in cd
  pub luminance_intensity: Vec3<f32>,
  pub position: HighPrecisionTranslationUniform,
  pub cutoff_distance: f32,
}

pub fn use_point_uniform_array(cx: &mut QueryGPUHookCx) -> UniformArray<PointLightUniform, 8> {
  let (cx, uniform) = cx.use_uniform_array_buffers();

  let offset = offset_of!(PointLightUniform, luminance_intensity);
  cx.use_changes::<PointLightIntensity>()
    .update_uniform_array(uniform, offset, cx.gpu);

  let offset = offset_of!(PointLightUniform, cutoff_distance);
  cx.use_changes::<PointLightCutOffDistance>()
    .update_uniform_array(uniform, offset, cx.gpu);

  let offset = offset_of!(PointLightUniform, position);

  use_global_node_world_mat(cx)
    .fanout(cx.use_db_rev_ref_tri_view::<PointLightRefNode>())
    .into_delta_change()
    .map(|change| change.collective_map(|mat| into_hpt(mat.position()).into_storage()))
    .use_assure_result(cx)
    .update_uniform_array(uniform, offset, cx.gpu);

  uniform.clone()
}
