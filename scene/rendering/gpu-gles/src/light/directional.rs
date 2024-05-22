use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct DirectionalLightUniform {
  /// in lx
  pub illuminance: Vec3<f32>,
  pub direction: Vec3<f32>,
}

pub fn directional_uniform_array(
  gpu: &GPUResourceCtx,
) -> UniformArrayUpdateContainer<DirectionalLightUniform> {
  let buffer = UniformBufferDataView::create_default(&gpu.device);

  let illuminance = global_watch()
    .watch::<DirectionalLightIlluminance>()
    .into_uniform_array_collection_update(offset_of!(DirectionalLightUniform, illuminance), gpu);

  let direction = scene_node_derive_world_mat()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<DirectionalRefNode>())
    .collective_map(|mat| mat.forward().reverse().normalize())
    .into_uniform_array_collection_update(offset_of!(DirectionalLightUniform, direction), gpu);

  UniformArrayUpdateContainer::new(buffer)
    .with_source(illuminance)
    .with_source(direction)
}
