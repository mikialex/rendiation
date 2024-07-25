use std::any::Any;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::Weak;

use parking_lot::RwLock;
use rendiation_algebra::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod backend;
pub use backend::*;
mod api;
pub use api::*;

pub struct GPURaytracingDevice {
  pub pipeline: Box<dyn GPURayTracingPipelineDeviceProvider>,
  pub acceleration_structure: Box<dyn GPURayTracingAccelerationStructureDeviceProvider>,
}

pub struct GPUAccelerationStructure {
  pub internal: Box<dyn GPUAccelerationStructureProvider>,
}
