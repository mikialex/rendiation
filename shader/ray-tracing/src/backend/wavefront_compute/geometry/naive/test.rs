use crate::backend::wavefront_compute::geometry::naive::*;

pub(crate) fn init_default_acceleration_structure(
  system: &dyn GPUAccelerationStructureSystemProvider,
) {
  #[rustfmt::skip]
  const CUBE_POSITION: [f32; 72] = [
     0.5,  0.5,  0.5, -0.5,  0.5,  0.5, -0.5, -0.5,  0.5,  0.5, -0.5,  0.5, // v0,v1,v2,v3 (front)
     0.5,  0.5,  0.5,  0.5, -0.5,  0.5,  0.5, -0.5, -0.5,  0.5,  0.5, -0.5, // v0,v3,v4,v5 (right)
     0.5,  0.5,  0.5,  0.5,  0.5, -0.5, -0.5,  0.5, -0.5, -0.5,  0.5,  0.5, // v0,v5,v6,v1 (top)
    -0.5,  0.5,  0.5, -0.5,  0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5,  0.5, // v1,v6,v7,v2 (left)
    -0.5, -0.5, -0.5,  0.5, -0.5, -0.5,  0.5, -0.5,  0.5, -0.5, -0.5,  0.5, // v7,v4,v3,v2 (bottom)
     0.5, -0.5, -0.5, -0.5, -0.5, -0.5, -0.5,  0.5, -0.5,  0.5,  0.5, -0.5, // v4,v7,v6,v5 (back)
  ];
  #[rustfmt::skip]
  const CUBE_INDEX: [u16; 36] = [
     0, 1, 2,   2, 3, 0,    // v0-v1-v2, v2-v3-v0 (front)
     4, 5, 6,   6, 7, 4,    // v0-v3-v4, v4-v5-v0 (right)
     8, 9,10,  10,11, 8,    // v0-v5-v6, v6-v1-v0 (top)
    12,13,14,  14,15,12,    // v1-v6-v7, v7-v2-v1 (left)
    16,17,18,  18,19,16,    // v7-v4-v3, v3-v2-v7 (bottom)
    20,21,22,  22,23,20,    // v4-v7-v6, v6-v5-v4 (back)
  ];

  let blas_handle = system.create_bottom_level_acceleration_structure(&[
    BottomLevelAccelerationStructureBuildSource {
      flags: GEOMETRY_FLAG_OPAQUE,
      geometry: BottomLevelAccelerationStructureBuildBuffer::Triangles {
        positions: CUBE_POSITION
          .chunks_exact(3)
          .map(|abc| vec3(abc[0], abc[1], abc[2]))
          .collect(),
        indices: CUBE_INDEX.map(|i| i as u32).into_iter().collect(),
      },
    },
  ]);

  fn add_tlas_source(
    vec: &mut Vec<TopLevelAccelerationStructureSourceInstance>,
    transform: Mat4<f32>,
    blas_handle: &BottomLevelAccelerationStructureHandle,
  ) {
    vec.push(TopLevelAccelerationStructureSourceInstance {
      transform,
      instance_custom_index: 0,
      mask: u32::MAX,
      instance_shader_binding_table_record_offset: 0,
      flags: 0,
      acceleration_structure_handle: *blas_handle,
    });
  }

  let mut sources0 = vec![];
  for i in -2..=2 {
    for j in -2..=2 {
      add_tlas_source(
        &mut sources0,
        Mat4::translate((i as f32 * 1.5, j as f32 * 1.5, -10.)),
        &blas_handle,
      );
    }
  }
  add_tlas_source(
    &mut sources0,
    Mat4::translate((0., 4.5, -10.)) * Mat4::scale((5., 1., 1.)),
    &blas_handle,
  );
  add_tlas_source(
    &mut sources0,
    Mat4::translate((0., -4.5, -10.))
      * Mat4::rotate_y(std::f32::consts::PI)
      * Mat4::scale((5., 1., 1.)),
    &blas_handle,
  );
  add_tlas_source(
    &mut sources0,
    Mat4::translate((4.5, -4.5, -10.))
      * Mat4::rotate_y(std::f32::consts::PI * 0.5)
      * Mat4::scale((5., 1., 1.)),
    &blas_handle,
  );
  add_tlas_source(
    &mut sources0,
    Mat4::translate((-4.5, -4.5, -10.))
      * Mat4::rotate_y(std::f32::consts::PI * -0.5)
      * Mat4::scale((5., 1., 1.)),
    &blas_handle,
  );

  let _tlas0 = system.create_top_level_acceleration_structure(&sources0);

  // let mut sources1 = vec![];
  // for i in -2..=2 {
  //   for j in -2..=2 {
  //     for k in -2..=2 {
  //       add_tlas_source(
  //         &mut sources1,
  //         Mat4::translate((i as f32 * 2., j as f32 * 2., -10. + k as f32 * 2.)),
  //         &blas_handle,
  //       );
  //     }
  //   }
  // }
  //
  // let _tlas1 = system.create_top_level_acceleration_structure(&sources1);
}

#[test]
fn test_both_triangle() {
  test_gpu_triangle();
  test_cpu_triangle();
}

#[test]
fn test_cpu_triangle() {
  const W: usize = 256;
  const H: usize = 256;
  const FAR: f32 = 100.;
  const ORIGIN: Vec3<f32> = vec3(0., 0., 0.);
  // const GEOMETRY_IDX_MAX: u32 = 1;
  const PRIMITIVE_IDX_MAX: u32 = 12;

  let (gpu, _) = futures::executor::block_on(GPU::new(Default::default())).unwrap();
  let system = NaiveSahBVHSystem::new(gpu);
  init_default_acceleration_structure(&system);

  let _ = system.get_or_build_gpu_data(); // trigger build
  let inner = system.inner.read().unwrap();
  let cpu_data = inner.cpu_data.as_ref().unwrap();

  let mut payload = ShaderRayTraceCallStoragePayload::zeroed();
  payload.ray_flags = RayFlagConfigRaw::RAY_FLAG_CULL_BACK_FACING_TRIANGLES as u32;
  payload.cull_mask = u32::MAX;
  payload.range = vec2(0., FAR);
  payload.tlas_idx = 0;
  payload.ray_origin = ORIGIN;

  let mut out = Box::new([[(FAR, 0); W]; H]);

  for j in 0..H {
    for i in 0..W {
      let x = (i as f32 + 0.5) / W as f32 * 2. - 1.;
      let y = 1. - (j as f32 + 0.5) / H as f32 * 2.;
      let target = vec3(x, y, -1.); // fov = 90 deg
      let direction = (target - ORIGIN).normalize();

      payload.ray_direction = direction;
      cpu_data.traverse(
        &payload,
        &mut |_geometry_id, primitive_id, distance, _position| {
          let (d, id) = &mut out[j][i];
          if distance < *d {
            *d = distance;
            *id = primitive_id + 1;
            return true;
          }
          false
        },
      );
    }
  }
  println!(
    "tri visit count: {}",
    TRI_VISIT_COUNT.load(std::sync::atomic::Ordering::Relaxed)
  );
  println!(
    "tri hit count: {}",
    TRI_HIT_COUNT.load(std::sync::atomic::Ordering::Relaxed)
  );
  println!(
    "bvh visit count: {}",
    BVH_VISIT_COUNT.load(std::sync::atomic::Ordering::Relaxed)
  );
  println!(
    "bvh hit count: {}",
    BVH_HIT_COUNT.load(std::sync::atomic::Ordering::Relaxed)
  );

  let mut file = format!("P2\n{W} {H}\n{PRIMITIVE_IDX_MAX}\n");
  for j in 0..H {
    file.push_str(out[j].map(|(_, id)| format!("{id}")).join(" ").as_str());
    file.push('\n');
  }
  std::fs::write("trace_cpu.pbm", file).unwrap();
}

#[test]
fn test_gpu_triangle() {
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
  std::fs::write("trace_gpu.pbm", file).unwrap();

  #[derive(Clone)]
  struct GpuTester {
    upstream: Box<dyn DeviceParallelCompute<Node<u32>>>,
    system: NaiveSahBVHSystem,
  }
  struct GpuTesterInner {
    upstream: Box<dyn DeviceInvocationComponent<Node<u32>>>,
    system: NaiveSahBvhGpu,
  }

  impl GpuTester {
    fn new(upstream: Box<dyn DeviceParallelCompute<Node<u32>>>, gpu: GPU) -> Self {
      let system = NaiveSahBVHSystem::new(gpu);
      init_default_acceleration_structure(&system);
      Self { upstream, system }
    }
  }
  impl DeviceParallelCompute<Node<u32>> for GpuTester {
    fn execute_and_expose(
      &self,
      cx: &mut DeviceParallelComputeCtx,
    ) -> Box<dyn DeviceInvocationComponent<Node<u32>>> {
      Box::new(GpuTesterInner {
        upstream: self.upstream.execute_and_expose(cx),
        system: self.system.get_or_build_gpu_data().clone(),
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
      builder.log_result = true;

      let upstream_shader = self.upstream.build_shader(builder);

      let traversable = self.system.clone().build_shader(builder);

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
          };

          // todo how to access user payload?
          let output =
            traversable.traverse(payload, &|_ctx, _reporter| {}, &|_ctx| val(ACCEPT_HIT));
          (
            output.payload.hit_ctx.primitive_id + output.is_some.into_u32(),
            valid,
          )
        })
        .into_boxed()
    }
    fn bind_input(&self, builder: &mut BindingBuilder) {
      self.upstream.bind_input(builder);
      self.system.bind_pass(builder);
    }
    fn requested_workgroup_size(&self) -> Option<u32> {
      self.upstream.requested_workgroup_size()
    }
  }
}
