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
    let device_support_ray_query = gpu.info.supported_features.contains(
      Features::EXPERIMENTAL_RAY_QUERY | Features::EXPERIMENTAL_RAY_TRACING_ACCELERATION_STRUCTURE,
    );
    Self {
      gpu: gpu.clone(),
      tlas_sys: if device_support_ray_query {
        Box::new(HardwareInlineRayQuerySystem::new(gpu.clone()))
      } else {
        Box::new(NaiveSahBVHSystem::new(gpu.clone()))
      },
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
      ray_gen_spawner: RangedTaskSpawner {
        size_offset: create_uniform(Vec4::zero(), &self.gpu.device),
      },
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
  #[allow(dead_code)]
  ray_gen_spawner: RangedTaskSpawner,
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

    let tile_size = 512; // todo, tile size should computed by memory limits
    let required_size = (tile_size * tile_size) as usize;

    let tlas_sys = self.tlas_sys.create_comp_instance(&mut cx);
    let (executor, task_source) = executor.get_or_compile_task_executor_and_task_source(
      &mut cx,
      source,
      tlas_sys,
      self.sbt_sys.clone(),
      required_size as u32,
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

    let (x, y, z) = size;
    assert_eq!(z, 1); // todo, support z;

    let graph_executor = &mut executor.graph_executor;

    for RectRange { offset, size } in rect_split_iter((x, y), tile_size * tile_size) {
      // todo, queue write buffer seems not take effect even if we submit queue, check if it's a wgpu bug?
      // self.ray_gen_spawner.size_offset.write_at(
      //   &self.gpu.queue,
      //   &Vec4::new(size.0, size.1, offset.0, offset.1),
      //   0,
      // );
      // cx.submit_recorded_work_and_continue();

      let ray_gen_spawner = RangedTaskSpawner {
        size_offset: create_uniform(
          Vec4::new(size.0, size.1, offset.0, offset.1),
          &self.gpu.device,
        ),
      };

      graph_executor.dispatch_allocate_init_task::<Vec3<u32>>(
        &mut cx,
        size.0 * size.1,
        executor.resource.info.ray_gen_task_idx,
        &ray_gen_spawner,
      );

      graph_executor.execute(&mut cx, source.execution_round_hint as usize, &task_source);
    }
  }
}

struct RangedTaskSpawner {
  size_offset: UniformBufferDataView<Vec4<u32>>,
}
impl ShaderHashProvider for RangedTaskSpawner {
  shader_hash_type_id! {}
}
impl TaskSpawner<Vec3<u32>> for RangedTaskSpawner {
  fn build_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn TaskSpawnerInvocation<Vec3<u32>>> {
    Box::new(RangedTaskSpawnerInvocation {
      size_offset: cx.bind_by(&self.size_offset),
    })
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.size_offset);
  }
}
struct RangedTaskSpawnerInvocation {
  size_offset: ShaderReadonlyPtrOf<Vec4<u32>>,
}

impl TaskSpawnerInvocation<Vec3<u32>> for RangedTaskSpawnerInvocation {
  fn spawn_task(&self, global_id: Node<u32>, _count: Node<u32>) -> Node<Vec3<u32>> {
    let size_offset = self.size_offset.load();
    let width = size_offset.x();
    let offset = size_offset.zw();
    let x = global_id % width;
    let y = global_id / width;
    (x + offset.x(), y + offset.y(), val(0)).into()
  }
}

pub fn rect_split_iter(full_size: (u32, u32), max_area: u32) -> impl Iterator<Item = RectRange> {
  let full_area = full_size.0 * full_size.1;
  let split_count = full_area / max_area + 1;

  let sub_width = full_size.0 / split_count;

  (0..split_count).map(move |i| RectRange {
    offset: (sub_width * i, 0),
    size: (sub_width, full_size.1),
  })
}

// we currently not use this because it yield small unnecessary edge tile.
pub fn tiling_iter(full_size: (u32, u32), tile_size: u32) -> impl Iterator<Item = RectRange> {
  let x_repeat = full_size.0 / tile_size + 1;
  let y_repeat = full_size.1 / tile_size + 1;

  (0..x_repeat)
    .flat_map(move |x| (0..y_repeat).map(move |y| (x, y)))
    .map(move |(x, y)| {
      let offset = (x * tile_size, y * tile_size);
      let x_left = full_size.0 - offset.0;
      let y_left = full_size.1 - offset.1;

      RectRange {
        offset,
        size: (tile_size.min(x_left), tile_size.min(y_left)),
      }
    })
}

#[derive(Debug)]
pub struct RectRange {
  pub offset: (u32, u32),
  pub size: (u32, u32), // size may smaller than tile_size
}
