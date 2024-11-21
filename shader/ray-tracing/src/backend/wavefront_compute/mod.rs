mod geometry;
pub use geometry::*;
mod sbt;
use parking_lot::RwLock;
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

pub struct GPUWaveFrontComputeRaytracingDevice {
  sbt_sys: ShaderBindingTableDeviceInfo,
}

impl GPURayTracingDeviceProvider for GPUWaveFrontComputeRaytracingDevice {
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

  fn create_raytracing_pipeline_executor(&self) -> GPURaytracingPipelineExecutor {
    GPURaytracingPipelineExecutor {
      inner: Box::new(GPUWaveFrontComputeRaytracingBakedPipeline {
        inner: Arc::new(RwLock::new(
          GPUWaveFrontComputeRaytracingBakedPipelineInner { executor: None },
        )),
      }),
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
    pipeline_source: &GPURaytracingPipelineAndBindingSource,
    executor: &GPURaytracingPipelineExecutor,
    size: (u32, u32, u32),
    sbt: &dyn ShaderBindingTableProvider,
  ) {
    let executor = executor
      .inner
      .access_impl()
      .downcast_ref::<GPUWaveFrontComputeRaytracingBakedPipeline>()
      .unwrap();
    let mut pipeline = executor.inner.write();

    let sbt = sbt
      .access_impl()
      .downcast_ref::<ShaderBindingTableInfo>()
      .unwrap();

    let mut encoder = self.gpu.create_encoder();
    let mut cx = DeviceParallelComputeCtx::new(&self.gpu, &mut encoder);

    let required_size = (size.0 * size.1 * size.2) as usize;

    let pipeline = pipeline.get_or_compile_executor(
      &mut cx,
      pipeline_source,
      self.tlas_sys.create_comp_instance(),
      self.sbt_sys.clone(),
      required_size,
    );

    // setup current binding sbt:
    pipeline
      .target_sbt_buffer
      .write_at(0, Std430::as_bytes(&sbt.self_idx), &self.gpu.queue);

    // setup launch size:
    pipeline.launch_size_buffer.write_at(
      0,
      Std430::as_bytes(&vec3(size.0, size.1, size.2)),
      &self.gpu.queue,
    );

    {
      let executor = &mut pipeline.graph;

      executor.dispatch_allocate_init_task(
        &mut cx,
        required_size as u32,
        pipeline.ray_gen_task_idx,
        // ray-gen payload is linear id. see TracingCtxProviderFutureInvocation.
        |global_id| global_id,
      );
    }

    let round_count = 3; // todo;
    for _ in 0..round_count {
      pipeline.graph.execute(&mut cx, 1);
    }
  }
}
