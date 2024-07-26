mod native;
mod wavefront_compute;
pub use native::*;
pub use wavefront_compute::*;

use crate::*;

pub trait GPURaytracingPipelineProvider {
  fn compile(&self, desc: GPURaytracingPipelineBuilder) -> u32;
}

pub trait RayTracingPassEncoderProvider {
  fn set_pipeline(&self, pipeline: u32);
  fn set_bindgroup(&self, index: u32, bindgroup: &rendiation_webgpu::BindGroup);
  fn trace_ray(
    &self,
    size: (u32, u32, u32),
    ray_gen: &RayGenShaderBindingTable,
    ray_miss: &RayMissShaderBindingTable,
    ray_hit: &RayHitShaderBindingTable,
  );
}

pub trait GPURayTracingPipelineDeviceProvider {
  fn create_raytracing_pipeline(
    &self,
    desc: &GPURaytracingPipelineBuilder,
  ) -> Box<dyn GPURaytracingPipelineProvider>;
}

pub struct RayGenShaderBindingTable;
pub struct RayMissShaderBindingTable;
pub struct RayHitShaderBindingTable {}

impl RayHitShaderBindingTable {
  pub fn set_record(&self, idx: u32, shader_handle: u32, parameter: &[u8]) {
    //
  }
}
