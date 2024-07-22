mod mega_kernel;
mod native;
mod wavefront_kernel;
// pub use mega_kernel::*;
// pub use native::*;
// pub use wavefront_kernel::*;

use crate::*;

pub trait GPURaytracingPipelineProvider {}

pub trait RayTracingPassEncoderProvider {
  fn set_pipeline(&self, pipeline: &dyn GPURaytracingPipelineProvider);
  fn set_bindgroup(&self, index: u32, bindgroup: &rendiation_webgpu::BindGroup);
  fn trace_ray(&self, size: (u32, u32, u32));
}

pub trait GPURayTracingPipelineDeviceProvider {
  fn create_raytracing_pipeline(
    &self,
    desc: &GPURaytracingPipelineBuilder,
  ) -> Box<dyn GPURaytracingPipelineProvider>;
}
