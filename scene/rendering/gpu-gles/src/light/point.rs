use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct PointLightUniform {
  /// in cd
  pub luminance_intensity: Vec3<f32>,
  pub position: Vec3<f32>,
  pub cutoff_distance: f32,
}

pub fn point_uniform_array(gpu: &GPU) -> UniformArrayUpdateContainer<PointLightUniform> {
  let buffer = UniformBufferDataView::create_default(&gpu.device);

  let luminance_intensity = global_watch()
    .watch::<PointLightIntensity>()
    .into_uniform_array_collection_update(offset_of!(PointLightUniform, luminance_intensity), gpu);

  let cutoff_distance = global_watch()
    .watch::<PointLightCutOffDistance>()
    .into_uniform_array_collection_update(offset_of!(PointLightUniform, cutoff_distance), gpu);

  let position = scene_node_derive_world_mat()
    .one_to_many_fanout(global_rev_ref().watch_inv_ref::<PointLightRefNode>())
    .collective_map(|mat| mat.position())
    .into_uniform_array_collection_update(offset_of!(PointLightUniform, position), gpu);

  UniformArrayUpdateContainer::new(buffer)
    .with_source(luminance_intensity)
    .with_source(cutoff_distance)
    .with_source(position)
}
