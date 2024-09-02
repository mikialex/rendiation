mod geometry;
pub use geometry::*;
mod sbt;
pub use sbt::*;
mod trace_task;
pub use trace_task::*;
mod pipeline;
pub use pipeline::*;
mod ctx;
pub use ctx::*;

use crate::*;

pub struct GPUWaveFrontComputeRaytracingSystem {
  gpu: GPU,
  tlas_sys: Box<dyn GPUAccelerationStructureSystemProvider>,
}

impl GPURaytracingSystem for GPUWaveFrontComputeRaytracingSystem {
  fn create_raytracing_device(&self) -> Box<dyn GPURayTracingDeviceProvider> {
    Box::new(GPUWaveFrontComputeRaytracingDevice {
      gpu: self.gpu.clone(),
      default_init_size: todo!(),
      sbt_sys: todo!(),
      tlas_sys: self.tlas_sys.clone(),
    })
  }

  fn create_raytracing_encoder(&self) -> Box<dyn RayTracingPassEncoderProvider> {
    Box::new(GPUWaveFrontComputeRaytracingEncoder {
      gpu: self.gpu.clone(),
      current_pipeline: None,
    })
  }

  fn create_acceleration_structure_system(
    &self,
  ) -> Box<dyn GPUAccelerationStructureSystemProvider> {
    self.tlas_sys.clone()
  }
}

pub struct GPUWaveFrontComputeRaytracingDevice {
  gpu: GPU,
  default_init_size: usize,
  sbt_sys: ShaderBindingTableDeviceInfo,
  tlas_sys: Box<dyn GPUAccelerationStructureSystemProvider>,
}

impl GPURayTracingDeviceProvider for GPUWaveFrontComputeRaytracingDevice {
  fn create_raytracing_pipeline(
    &self,
    desc: &GPURaytracingPipelineDescriptor,
  ) -> Box<dyn GPURaytracingPipelineProvider> {
    let mut encoder = self.gpu.create_encoder();
    let mut pass = encoder.begin_compute_pass();
    let r = Box::new(GPUWaveFrontComputeRaytracingBakedPipelineInner::compile(
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
      self_idx: todo!(),
    })
  }
}

pub struct GPUWaveFrontComputeRaytracingEncoder {
  gpu: GPU,
  current_pipeline: Option<GPUWaveFrontComputeRaytracingBakedPipeline>,
}

impl RayTracingPassEncoderProvider for GPUWaveFrontComputeRaytracingEncoder {
  fn set_pipeline(&mut self, pipeline: &dyn GPURaytracingPipelineProvider) {
    self.current_pipeline = pipeline
      .access_impl()
      .downcast_ref::<GPUWaveFrontComputeRaytracingBakedPipeline>()
      .unwrap()
      .clone()
      .into();
  }

  fn trace_ray(&mut self, size: (u32, u32, u32), sbt: &dyn ShaderBindingTableProvider) {
    let current_pipeline = self.current_pipeline.as_ref().expect("no pipeline bound");
    let mut current_pipeline = current_pipeline.inner.write();

    let sbt = sbt
      .access_impl()
      .downcast_ref::<ShaderBindingTableInfo>()
      .unwrap();

    let mut cx = DeviceParallelComputeCtx::new(&self.gpu);

    // setup sbt:
    current_pipeline.target_sbt_buffer.write_at(
      0,
      Std430::as_bytes(&sbt.self_idx),
      &self.gpu.queue,
    );

    current_pipeline.graph.set_execution_size(
      &self.gpu,
      &mut cx,
      (size.0 * size.1 * size.2) as usize,
    );
    current_pipeline.graph.execute(&mut cx, todo!());
  }
}
