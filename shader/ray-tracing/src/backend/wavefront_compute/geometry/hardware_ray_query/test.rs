#[test]
fn test_gpu_triangle() {
  use rendiation_algebra::{vec2, vec3, Vec3};
  use rendiation_device_parallel_compute::{
    DeviceInvocation, DeviceInvocationComponent, DeviceInvocationExt, DeviceParallelCompute,
    DeviceParallelComputeCtx, DeviceParallelComputeIO, DeviceParallelComputeIOExt,
  };
  use rendiation_shader_api::{val, Node, ShaderComputePipelineBuilder};
  use rendiation_webgpu::{
    create_gpu_read_write_storage, shader_hash_type_id, BindingBuilder, PipelineHasher,
    ShaderHashProvider, StorageBufferDataView, ZeroedArrayByArrayLength, GPU,
  };
  use rendiation_webgpu_virtual_buffer::ComputeShaderBuilderAbstractBufferExt;

  use crate::backend::{init_default_acceleration_structure, TEST_ANYHIT_BEHAVIOR};
  use crate::{
    GPUAccelerationStructureSystemCompImplInstance, HardwareInlineRayQueryInstance,
    HardwareInlineRayQuerySystem, RayFlagConfigRaw,
    ShaderRayTraceCallStoragePayloadShaderAPIInstance,
  };

  const H: usize = 256;
  const W: usize = 256;
  const FAR: f32 = 100.;
  const ORIGIN: Vec3<f32> = vec3(0., 0., 0.);
  // const GEOMETRY_IDX_MAX: u32 = 1;
  const PRIMITIVE_IDX_MAX: u32 = 12;

  let dummy_array = vec![0u32; H * W];

  let (gpu, _) = futures::executor::block_on(GPU::new(Default::default())).unwrap();
  let mut encoder = gpu.create_encoder();
  let mut cx = DeviceParallelComputeCtx::new(&gpu, &mut encoder);

  let direction = Box::new(dummy_array) as Box<dyn DeviceParallelCompute<Node<u32>>>;
  let tester = GpuTester::new(direction, gpu);

  cx.force_indirect_dispatch = false;
  let (_, _size, result) = futures::executor::block_on(tester.read_back_host(&mut cx)).unwrap();
  // println!("result {:?} {:?}", result.len(), result);

  let mut file = format!("P2\n{W} {H}\n{PRIMITIVE_IDX_MAX}\n");
  for j in 0..H {
    file.push_str(
      result[j * W..(j + 1) * W]
        .iter()
        .map(|v| format!("{v}"))
        .collect::<Vec<_>>()
        .join(" ")
        .as_str(),
    );
    file.push('\n');
  }
  std::fs::write("trace_ray_query_backend.pbm", file).unwrap();

  #[derive(Clone)]
  struct GpuTester {
    upstream: Box<dyn DeviceParallelCompute<Node<u32>>>,
    payloads: StorageBufferDataView<[u32]>,
    system: HardwareInlineRayQuerySystem,
  }
  struct GpuTesterInner {
    upstream: Box<dyn DeviceInvocationComponent<Node<u32>>>,
    payloads: StorageBufferDataView<[u32]>,
    system: HardwareInlineRayQueryInstance,
  }

  impl GpuTester {
    fn new(upstream: Box<dyn DeviceParallelCompute<Node<u32>>>, gpu: GPU) -> Self {
      let payloads = create_gpu_read_write_storage::<[u32]>(ZeroedArrayByArrayLength(1), &gpu);
      let system = HardwareInlineRayQuerySystem::new(gpu.clone());

      init_default_acceleration_structure(&system);
      let mut encoder = gpu.create_encoder();
      system.maintain(&mut encoder);
      gpu.queue.submit_encoder(encoder);

      Self {
        upstream,
        system,
        payloads,
      }
    }
  }
  impl DeviceParallelCompute<Node<u32>> for GpuTester {
    fn execute_and_expose(
      &self,
      cx: &mut DeviceParallelComputeCtx,
    ) -> Box<dyn DeviceInvocationComponent<Node<u32>>> {
      Box::new(GpuTesterInner {
        upstream: self.upstream.execute_and_expose(cx),
        system: self.system.create_instance(),
        payloads: self.payloads.clone(),
      })
    }
    fn result_size(&self) -> u32 {
      self.upstream.result_size()
    }
  }
  impl DeviceParallelComputeIO<u32> for GpuTester {}

  impl ShaderHashProvider for GpuTesterInner {
    fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
      self.upstream.hash_pipeline_with_type_info(hasher)
    }
    shader_hash_type_id! {}
  }
  impl DeviceInvocationComponent<Node<u32>> for GpuTesterInner {
    fn work_size(&self) -> Option<u32> {
      self.upstream.work_size()
    }
    fn build_shader(
      &self,
      builder: &mut ShaderComputePipelineBuilder,
    ) -> Box<dyn DeviceInvocation<Node<u32>>> {
      // builder.log_result = true;

      let upstream_shader = self.upstream.build_shader(builder);

      let traversable = self.system.build_shader(builder);
      let payloads = builder.bind_abstract_storage(&self.payloads);

      upstream_shader
        .adhoc_invoke_with_self_size(move |upstream, id| {
          let (_, valid) = upstream.invocation_logic(id);

          let linear_idx = id.x();
          let idx_x = linear_idx % val(W as u32);
          let idx_y = linear_idx / val(W as u32);
          let launch_id: Node<Vec3<u32>> = (idx_x, idx_y, val(0)).into();
          let launch_size: Node<Vec3<u32>> = (val(W as u32), val(H as u32), val(1)).into();

          let x =
            (launch_id.x().into_f32() + val(0.5)) / launch_size.x().into_f32() * val(2.) - val(1.);
          let y =
            val(1.) - (launch_id.y().into_f32() + val(0.5)) / launch_size.y().into_f32() * val(2.);
          let target: Node<Vec3<f32>> = (x, y, val(-1.)).into(); // fov = 90 deg
          let dir = (target - val(ORIGIN)).normalize();

          let ray_flags = RayFlagConfigRaw::RAY_FLAG_CULL_BACK_FACING_TRIANGLES as u32;
          let payload = ShaderRayTraceCallStoragePayloadShaderAPIInstance {
            launch_id,
            launch_size,
            payload_ref: val(0),
            tlas_idx: val(0),
            ray_flags: val(ray_flags),
            cull_mask: val(u32::MAX),
            sbt_ray_config_offset: val(0),
            sbt_ray_config_stride: val(0),
            miss_index: val(0),
            ray_origin: val(ORIGIN),
            ray_direction: dir,
            range: val(vec2(0.01, FAR)),
            payload_u32_len: val(1),
          };

          let output =
            traversable.traverse(payload, payloads.clone(), &|_ctx, _reporter| {}, &|_ctx| {
              val(TEST_ANYHIT_BEHAVIOR)
            });
          (
            output.is_some.into_u32()
              * (output.payload.hit_ctx.primitive_id % val(PRIMITIVE_IDX_MAX) + val(1)),
            valid,
          )
        })
        .into_boxed()
    }
    fn bind_input(&self, builder: &mut BindingBuilder) {
      self.upstream.bind_input(builder);
      self.system.bind_pass(builder);
      builder.bind(&self.payloads);
    }
    fn requested_workgroup_size(&self) -> Option<u32> {
      self.upstream.requested_workgroup_size()
    }
  }
}
