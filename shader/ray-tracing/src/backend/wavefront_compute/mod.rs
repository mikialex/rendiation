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
      default_init_size: 512 * 512,
      sbt_sys: ShaderBindingTableDeviceInfo::new(&self.gpu),
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
    let mut cx = DeviceParallelComputeCtx::new(&self.gpu);
    Box::new(GPUWaveFrontComputeRaytracingBakedPipelineInner::compile(
      &mut cx,
      self.tlas_sys.create_comp_instance(),
      self.sbt_sys.clone(),
      desc,
      self.default_init_size,
    ))
  }

  fn create_sbt(
    &self,
    mesh_count: u32,
    ray_type_count: u32,
  ) -> Box<dyn ShaderBindingTableProvider> {
    let self_idx = self.sbt_sys.allocate(mesh_count, ray_type_count).unwrap();
    Box::new(ShaderBindingTableInfo {
      sys: self.sbt_sys.clone(),
      self_idx,
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

    let size = (size.0 * size.1 * size.2) as usize;
    let executor = &mut current_pipeline.graph;
    executor.set_execution_size(&mut cx, size);
    executor.execute(&mut cx, size);
  }
}
