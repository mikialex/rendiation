mod native;
mod wavefront_compute;
pub use native::*;
pub use wavefront_compute::*;

use crate::*;

pub trait RayTracingPassEncoderProvider {
  fn set_pipeline(&self, pipeline: u32);
  fn set_bindgroup(&self, index: u32, bindgroup: &rendiation_webgpu::BindGroup);
  fn trace_ray(&self, size: (u32, u32, u32), sbt: &GPUShaderBindingTable);
}

pub trait GPURayTracingDeviceProvider {
  fn create_raytracing_pipeline(&self, desc: &GPURaytracingPipelineBuilder) -> u32;
  fn create_sbt(&self, desc: &GPURaytracingPipelineBuilder) -> Box<dyn ShaderBindingTableProvider>;
}

pub struct HitGroupShaderRecord {
  pub closet_hit: ShaderHandle,
  pub any_hit: Option<ShaderHandle>,
  pub intersection: Option<ShaderHandle>,
}

pub trait ShaderBindingTableProvider {
  fn resize(&mut self, mesh_count: u32, ray_type_count: u32);
  fn config_ray_generation(&mut self, s: ShaderHandle);
  fn config_hit_group(&mut self, mesh_idx: u32, hit_group: HitGroupShaderRecord);
  fn config_missing(&mut self, ray_ty_idx: u32, s: ShaderHandle);
}
