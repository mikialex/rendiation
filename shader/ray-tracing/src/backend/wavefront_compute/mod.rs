mod geometry;
pub use geometry::*;
mod sbt;
use parking_lot::lock_api::RwLock;
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
  sbt_sys: ShaderBindingTableDeviceInfo,
}

impl GPUWaveFrontComputeRaytracingSystem {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      gpu: gpu.clone(),
      tlas_sys: Box::new(NaiveSahBVHSystem::new(gpu.clone())),
      sbt_sys: ShaderBindingTableDeviceInfo::new(gpu),
    }
  }
}

impl GPURaytracingSystem for GPUWaveFrontComputeRaytracingSystem {
  fn create_tracer_base_builder(&self) -> TraceFutureBaseBuilder {
    TraceFutureBaseBuilder {
      inner: Arc::new(WaveFrontTracingBaseProvider),
    }
  }
  fn create_raytracing_device(&self) -> Box<dyn GPURayTracingDeviceProvider> {
    Box::new(GPUWaveFrontComputeRaytracingDevice {
      sbt_sys: self.sbt_sys.clone(),
    })
  }

  fn create_raytracing_encoder(&self) -> Box<dyn RayTracingPassEncoderProvider> {
    Box::new(GPUWaveFrontComputeRaytracingEncoder {
      gpu: self.gpu.clone(),
      current_pipeline: None,
      sbt_sys: self.sbt_sys.clone(),
      tlas_sys: self.tlas_sys.clone(),
    })
  }

  fn create_acceleration_structure_system(
    &self,
  ) -> Box<dyn GPUAccelerationStructureSystemProvider> {
    self.tlas_sys.clone()
  }
}

pub struct GPUWaveFrontComputeRaytracingDevice {
  sbt_sys: ShaderBindingTableDeviceInfo,
}

impl GPURayTracingDeviceProvider for GPUWaveFrontComputeRaytracingDevice {
  fn create_raytracing_pipeline(
    &self,
    desc: GPURaytracingPipelineDescriptor,
  ) -> Box<dyn GPURaytracingPipelineProvider> {
    Box::new(GPUWaveFrontComputeRaytracingBakedPipeline {
      inner: Arc::new(RwLock::new(
        GPUWaveFrontComputeRaytracingBakedPipelineInner {
          desc,
          executor: None,
        },
      )),
    })
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
  sbt_sys: ShaderBindingTableDeviceInfo,
  tlas_sys: Box<dyn GPUAccelerationStructureSystemProvider>,
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

    let mut encoder = self.gpu.create_encoder();
    let mut cx = DeviceParallelComputeCtx::new(&self.gpu, &mut encoder);

    let required_size = (size.0 * size.1 * size.2) as usize;

    let current_pipeline = current_pipeline.get_or_compile_executor(
      &mut cx,
      self.tlas_sys.create_comp_instance(),
      self.sbt_sys.clone(),
      required_size,
    );

    // setup current binding sbt:
    current_pipeline.target_sbt_buffer.write_at(
      0,
      Std430::as_bytes(&sbt.self_idx),
      &self.gpu.queue,
    );

    // setup launch size:
    current_pipeline.launch_size_buffer.write_at(
      0,
      Std430::as_bytes(&vec3(size.0, size.1, size.2)),
      &self.gpu.queue,
    );

    {
      let executor = &mut current_pipeline.graph;

      executor.dispatch_allocate_init_task(
        &mut cx,
        required_size as u32,
        current_pipeline.ray_gen_task_idx,
        // ray-gen payload is linear id. see TracingCtxProviderFutureInvocation.
        |global_id| global_id,
      );
    }

    let round_count = 3; // todo;
    for _ in 0..round_count {
      current_pipeline.graph.execute(&mut cx, 1);
    }
  }
}
