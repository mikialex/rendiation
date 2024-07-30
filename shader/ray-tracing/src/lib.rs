use rendiation_algebra::*;
use rendiation_device_task_graph::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod backend;
pub use backend::*;
mod api;
pub use api::*;

pub struct GPURaytracingDevice {
  pub pipeline: Box<dyn GPURayTracingPipelineDeviceProvider>,
  pub acceleration_structure: Box<dyn GPUAccelerationStructureInstanceBuilder>,
}
