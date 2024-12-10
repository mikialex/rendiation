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

#[derive(Clone)]
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

  fn create_raytracing_encoder(&self) -> Box<dyn RayTracingEncoderProvider> {
    Box::new(GPUWaveFrontComputeRaytracingEncoder {
      gpu: self.gpu.clone(),
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

#[derive(Clone)]
pub struct GPUWaveFrontComputeRaytracingDevice {
  sbt_sys: ShaderBindingTableDeviceInfo,
}

impl GPURayTracingDeviceProvider for GPUWaveFrontComputeRaytracingDevice {
  fn create_sbt(
    &self,
    max_geometry_count_in_blas: u32,
    max_tlas_offset: u32,
    ray_type_count: u32,
  ) -> Box<dyn ShaderBindingTableProvider> {
    let self_idx = self
      .sbt_sys
      .allocate(max_geometry_count_in_blas, max_tlas_offset, ray_type_count)
      .unwrap();
    Box::new(ShaderBindingTableInfo {
      sys: self.sbt_sys.clone(),
      self_idx,
      ray_stride: ray_type_count,
    })
  }

  fn create_raytracing_pipeline_executor(&self) -> GPURaytracingPipelineExecutor {
    GPURaytracingPipelineExecutor {
      inner: Box::new(GPUWaveFrontComputeRaytracingExecutorImpl::default()),
    }
  }
}

pub struct GPUWaveFrontComputeRaytracingEncoder {
  gpu: GPU,
  sbt_sys: ShaderBindingTableDeviceInfo,
  tlas_sys: Box<dyn GPUAccelerationStructureSystemProvider>,
}

impl RayTracingEncoderProvider for GPUWaveFrontComputeRaytracingEncoder {
  fn trace_ray(
    &mut self,
    source: &GPURaytracingPipelineAndBindingSource,
    executor: &GPURaytracingPipelineExecutor,
    size: (u32, u32, u32),
    sbt: &dyn ShaderBindingTableProvider,
  ) {
    let executor = executor
      .inner
      .access_impl()
      .downcast_ref::<GPUWaveFrontComputeRaytracingExecutorImpl>()
      .unwrap();
    let mut executor = executor.inner.write();

    let sbt = sbt
      .access_impl()
      .downcast_ref::<ShaderBindingTableInfo>()
      .unwrap();

    let mut encoder = self.gpu.create_encoder();
    let mut cx = DeviceParallelComputeCtx::new(&self.gpu, &mut encoder);

    let required_size = (size.0 * size.1 * size.2) as usize;

    let (executor, task_source) = executor.get_or_compile_task_executor_and_task_source(
      &mut cx,
      source,
      self.tlas_sys.create_comp_instance(),
      self.sbt_sys.clone(),
    );

    // setup current binding sbt:
    executor
      .resource
      .current_sbt
      .write_at(0, Std430::as_bytes(&sbt.self_idx), &self.gpu.queue);

    // setup launch size:
    executor.resource.launch_size.write_at(
      0,
      Std430::as_bytes(&vec3(size.0, size.1, size.2)),
      &self.gpu.queue,
    );

    {
      let graph_executor = &mut executor.graph_executor;

      graph_executor.dispatch_allocate_init_task(
        &mut cx,
        required_size as u32,
        executor.resource.info.ray_gen_task_idx,
        // ray-gen payload is linear id. see TracingCtxProviderFutureInvocation.
        |global_id| global_id,
      );
    }

    for _ in 0..source.execution_round_hint {
      executor.graph_executor.execute(&mut cx, 1, &task_source);
    }
  }
}
