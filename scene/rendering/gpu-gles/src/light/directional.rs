use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct DirectionalLightUniform {
  /// in lx
  pub illuminance: Vec3<f32>,
  pub direction: Vec3<f32>,
}

pub fn use_directional_uniform_array(
  cx: &mut QueryGPUHookCx,
) -> UniformArray<DirectionalLightUniform, 8> {
  let (cx, uniform) = cx.use_uniform_array_buffers();

  let offset = offset_of!(DirectionalLightUniform, illuminance);
  cx.use_changes::<DirectionalLightIlluminance>()
    .update_uniform_array(uniform, offset, cx.gpu);

  let offset = offset_of!(DirectionalLightUniform, direction);
  use_global_node_world_mat(cx)
    .fanout(cx.use_db_rev_ref_tri_view::<DirectionalRefNode>(), cx)
    .into_delta_change()
    .map(|change| change.collective_map(|mat| mat.forward().reverse().normalize().into_f32()))
    .use_assure_result(cx)
    .update_uniform_array(uniform, offset, cx.gpu);

  uniform.clone()
}
