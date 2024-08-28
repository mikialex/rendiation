mod geometry;
pub use geometry::*;
mod sbt;
pub use sbt::*;
mod trace_task;
pub use trace_task::*;
mod pipeline;
pub use pipeline::*;

use crate::*;

pub struct GPUWaveFrontComputeRaytracingSystem;

impl GPURaytracingSystem for GPUWaveFrontComputeRaytracingSystem {
  fn create_raytracing_device(&self) -> Box<dyn GPURayTracingDeviceProvider> {
    todo!()
  }

  fn create_raytracing_encoder(&self) -> Box<dyn RayTracingPassEncoderProvider> {
    todo!()
  }

  fn create_acceleration_structure_system(
    &self,
  ) -> Box<dyn GPUAccelerationStructureSystemProvider> {
    // NaiveSahBVHSystem
    todo!()
  }
}

pub struct GPUWaveFrontComputeRaytracingDevice {
  gpu: GPU,
  default_init_size: usize,
  sbt_sys: ShaderBindingTableDeviceInfo,
  tlas_sys: Box<dyn GPUAccelerationStructureSystemCompImplInstance>,
}

impl GPURayTracingDeviceProvider for GPUWaveFrontComputeRaytracingDevice {
  fn create_raytracing_pipeline(
    &self,
    desc: &GPURaytracingPipelineDescriptor,
  ) -> Box<dyn GPURaytracingPipelineProvider> {
    let mut encoder = self.gpu.create_encoder();
    let mut pass = encoder.begin_compute_pass();
    let r = Box::new(GPUWaveFrontComputeRaytracingBakedPipeline::compile(
      todo!(),
      self.sbt_sys.clone(),
      desc,
      &self.gpu.device,
      self.default_init_size,
      &mut pass,
    ));

    self.gpu.submit_encoder(encoder);

    r
  }

  fn create_sbt(&self) -> Box<dyn ShaderBindingTableProvider> {
    Box::new(ShaderBindingTableInfo {
      ray_generation: todo!(),
      ray_miss: todo!(),
      ray_hit: todo!(),
      sys: self.sbt_sys.clone(),
    })
  }
}

pub struct GPUWaveFrontComputeRaytracingEncoder {
  gpu: GPU,
  encoder: GPUCommandEncoder,
  current_pipeline: Option<GPUWaveFrontComputeRaytracingBakedPipeline>,
}

impl RayTracingPassEncoderProvider for GPUWaveFrontComputeRaytracingEncoder {
  fn set_pipeline(&self, pipeline: &dyn GPURaytracingPipelineProvider) {
    todo!()
  }

  fn set_bindgroup(&self, index: u32, bindgroup: &rendiation_webgpu::BindGroup) {
    let current_pipeline = self.current_pipeline.as_ref().expect("no pipeline bound");
    // copy buffer to buffer
    todo!()
  }

  fn trace_ray(&self, size: (u32, u32, u32), sbt: &dyn ShaderBindingTableProvider) {
    let current_pipeline = self.current_pipeline.as_ref().expect("no pipeline bound");

    let mut cx = DeviceParallelComputeCtx::new(&self.gpu);
    // current_pipeline.graph.execute(&mut cx, todo!());
    todo!()
  }
}
