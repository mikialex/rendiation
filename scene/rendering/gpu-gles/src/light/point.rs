use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct PointLightShaderInfo {
  pub luminance_intensity: Vec3<f32>,
  pub position: Vec3<f32>,
  pub cutoff_distance: f32,
}

pub fn point_uniform_array(
  gpu: &GPUResourceCtx,
) -> UniformArrayUpdateContainer<PointLightShaderInfo> {
  let buffer = UniformBufferDataView::create_default(&gpu.device);

  let luminance_intensity = global_watch()
    .watch_typed_key::<PointLightIntensity>()
    .into_uniform_array_collection_update(
      offset_of!(PointLightShaderInfo, luminance_intensity),
      gpu,
    );

  let cutoff_distance = global_watch()
    .watch_typed_key::<PointLightCutOffDistance>()
    .into_uniform_array_collection_update(offset_of!(PointLightShaderInfo, cutoff_distance), gpu);

  // todo

  UniformArrayUpdateContainer::new(buffer)
    .with_source(luminance_intensity)
    .with_source(cutoff_distance)
}
