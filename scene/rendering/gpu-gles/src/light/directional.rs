use crate::*;

#[repr(C)]
#[std140_layout]
#[derive(Copy, Clone, ShaderStruct, Default)]
pub struct DirectionalLightShaderInfo {
  /// in lx
  pub illuminance: Vec3<f32>,
  pub direction: Vec3<f32>,
}

pub fn directional_uniform_array(
  gpu: &GPUResourceCtx,
) -> UniformArrayUpdateContainer<DirectionalLightShaderInfo> {
  let buffer = UniformBufferDataView::create_default(&gpu.device);

  let illuminance = global_watch()
    .watch_typed_key::<DirectionalLightIlluminance>()
    .into_uniform_array_collection_update(offset_of!(DirectionalLightShaderInfo, illuminance), gpu);

  // todo direction

  UniformArrayUpdateContainer::new(buffer).with_source(illuminance)
}
