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
}

impl GPUWaveFrontComputeRaytracingSystem {
  pub fn new(gpu: &GPU) -> Self {
    Self {
      gpu: gpu.clone(),
      tlas_sys: Box::new(NaiveSahBVHSystem::new(gpu.clone())),
    }
  }
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
    let inner = GPUWaveFrontComputeRaytracingBakedPipelineInner::compile(
      &mut cx,
      self.tlas_sys.create_comp_instance(),
      self.sbt_sys.clone(),
      desc,
      self.default_init_size,
    );
    Box::new(GPUWaveFrontComputeRaytracingBakedPipeline {
      inner: Arc::new(RwLock::new(inner)),
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

    let w = size.0;
    let h = size.1;
    assert_eq!(size.2, 0);
    current_pipeline.launch_size_buffer.write_at(
      0,
      Std430::as_bytes(&vec3(w, h, 1)),
      &self.gpu.queue,
    );

    let size = (size.0 * size.1 * size.2) as usize;
    {
      let executor = &mut current_pipeline.graph;
      executor.resize_execution_size(&mut cx, size);
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
