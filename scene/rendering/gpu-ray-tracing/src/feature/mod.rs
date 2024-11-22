mod path_tracing;
pub use path_tracing::*;

mod ao;
pub use ao::*;

use crate::*;

#[derive(Clone)]
pub struct RtxSystemCore {
  pub rtx_system: Box<dyn GPURaytracingSystem>,
  pub rtx_device: Box<dyn GPURayTracingDeviceProvider>,
  pub rtx_acc: Box<dyn GPUAccelerationStructureSystemProvider>,
}

impl RtxSystemCore {
  pub fn new(rtx: Box<dyn GPURaytracingSystem>) -> Self {
    Self {
      rtx_device: rtx.create_raytracing_device(),
      rtx_acc: rtx.create_acceleration_structure_system(),
      rtx_system: rtx,
    }
  }
}
