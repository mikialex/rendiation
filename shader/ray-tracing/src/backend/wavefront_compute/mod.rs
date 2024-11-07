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

    let mut cx = DeviceParallelComputeCtx::new(&self.gpu);

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
    let w = size.0;
    let h = size.1;
    assert_eq!(size.2, 1);
    current_pipeline.launch_size_buffer.write_at(
      0,
      Std430::as_bytes(&vec3(w, h, 1)),
      &self.gpu.queue,
    );

    {
      let executor = &mut current_pipeline.graph;

      executor.dispatch_allocate_init_task(
        &mut cx,
        required_size as u32,
        current_pipeline.ray_gen_task_idx,
        |_| val(0_u32), // todo check if it's u32 payload if not define any payload
      );
    }

    let round_count = 3; // todo;
    for _ in 0..round_count {
      // reset read_back bumper;
      {
        let bumper = current_pipeline.tracer_read_back_bumper.read();
        cx.record_pass(|pass, device| {
          let hasher = shader_hasher_from_marker_ty!(SizeClear);
          let pipeline = device.get_or_cache_create_compute_pipeline(hasher, |mut builder| {
            builder.config_work_group_size(1);
            let current_size = builder.bind_by(&bumper.current_size);
            let bump_size = builder.bind_by(&bumper.bump_size);
            current_size.store(val(0));
            bump_size.atomic_store(val(0));
            builder
          });

          BindingBuilder::new_as_compute()
            .with_bind(&bumper.current_size)
            .with_bind(&bumper.bump_size)
            .setup_compute_pass(pass, device, &pipeline);

          pass.dispatch_workgroups(1, 1, 1);
        });
      }

      current_pipeline.graph.execute(&mut cx, 1);
    }
  }
}
