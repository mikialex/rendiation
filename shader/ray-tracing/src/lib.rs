use rendiation_algebra::*;
use rendiation_device_task_graph::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod backend;
pub use backend::*;
mod api;
use std::any::Any;

pub use api::*;

pub struct GPURaytracingDevice {
  pub pipeline: Box<dyn GPURayTracingDeviceProvider>,
  pub acceleration_structure: Box<dyn GPUAccelerationStructureInstanceBuilder>,
}

#[derive(Clone, Copy)]
pub struct DeviceOption<T> {
  pub is_some: Node<bool>,
  pub payload: T,
}
