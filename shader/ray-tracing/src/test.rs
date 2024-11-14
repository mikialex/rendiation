#[pollster::test]
async fn test_wavefront_compute() {
  use rendiation_texture_core::Size;

  let canvas_size = 64;

  use crate::*;
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();

  let texture_io_system = RayTracingTextureIO::default();

  pub struct RayTracingDebugOutput;
  impl RayTracingOutputTargetSemantic for RayTracingDebugOutput {}

  let debug_output = create_empty_2d_texture_view(
    &gpu,
    Size::from_u32_pair_min_one((canvas_size, canvas_size)),
    TextureUsages::all(),
    TextureFormat::Rgba8Unorm,
  );

  texture_io_system.install_output_target::<RayTracingDebugOutput>(debug_output);

  let system = GPUWaveFrontComputeRaytracingSystem::new(&gpu);
  let shader_base_builder = system.create_tracer_base_builder();
  let as_sys = system.create_acceleration_structure_system();

  use crate::GPURaytracingSystem;
  init_default_acceleration_structure(as_sys.as_ref());

  let rtx_device = system.create_raytracing_device();

  let mut rtx_pipeline_desc = GPURaytracingPipelineDescriptor::default();

  let ray_gen_shader = shader_base_builder
    .create_ray_gen_shader_base()
    .inject_ctx(texture_io_system.clone())
    .then_trace(
      // (&T, &mut TracingCtx) -> (Node<bool>, ShaderRayTraceCall, Node<P>)
      |_, ctx| {
        let ray_gen_ctx = ctx.ray_gen_ctx().unwrap();
        let launch_id = ray_gen_ctx.launch_id();
        let launch_size = ray_gen_ctx.launch_size();

        let tex_io = ctx.registry.get_mut::<FrameOutputInvocation>().unwrap();
        tex_io
          .write_output::<RayTracingDebugOutput>(launch_id.xy(), val(vec4(0., 0., 50. / 255., 1.)));

        const ORIGIN: Vec3<f32> = vec3(0., 0., 0.);
        let x =
          (launch_id.x().into_f32() + val(0.5)) / launch_size.x().into_f32() * val(2.) - val(1.);
        let y =
          val(1.) - (launch_id.y().into_f32() + val(0.5)) / launch_size.y().into_f32() * val(2.);
        let target: Node<Vec3<f32>> = (x, y, val(-1.)).into(); // fov = 90 deg
        let dir = (target - val(ORIGIN)).normalize();

        let ray_flags = RayFlagConfigRaw::RAY_FLAG_CULL_BACK_FACING_TRIANGLES as u32;
        let trace_call = ShaderRayTraceCall {
          tlas_idx: val(1),
          ray_flags: val(ray_flags),
          cull_mask: val(u32::MAX),
          sbt_ray_config: RaySBTConfig {
            offset: val(0),
            stride: val(0),
          },
          miss_index: val(0),
          ray: ShaderRay {
            origin: val(ORIGIN),
            direction: dir,
          },
          range: ShaderRayRange {
            min: val(0.1),
            max: val(100.),
          },
        };

        let ray_payload = ENode::<RayCustomPayload> { color: val(0) }.construct();

        (val(true), trace_call, ray_payload)
      },
    )
    .map(|(_, _payload), ctx| {
      let ray_gen_ctx = ctx.ray_gen_ctx().unwrap();
      let launch_id = ray_gen_ctx.launch_id();

      let tex_io = ctx.registry.get_mut::<FrameOutputInvocation>().unwrap();
      let prev = tex_io.read_output::<RayTracingDebugOutput>(launch_id.xy());
      tex_io.write_output::<RayTracingDebugOutput>(
        launch_id.xy(),
        (prev.x(), prev.y(), prev.z() + val(100. / 255.), val(1.)).into(),
      );
    });

  #[derive(Copy, Clone, Debug, Default, ShaderStruct)]
  pub struct RayCustomPayload {
    pub color: u32,
  }

  let ray_gen = rtx_pipeline_desc.register_ray_gen::<u32>(ray_gen_shader);
  let closest_hit = rtx_pipeline_desc.register_ray_closest_hit::<RayCustomPayload>(
    shader_base_builder
      .create_closest_hit_shader_base::<RayCustomPayload>()
      .inject_ctx(texture_io_system.clone())
      .map(|_, ctx| {
        let closest_ctx = ctx.closest_hit_ctx().unwrap();
        let launch_id = closest_ctx.launch_id();

        let tex_io = ctx.registry.get_mut::<FrameOutputInvocation>().unwrap();
        let prev = tex_io.read_output::<RayTracingDebugOutput>(launch_id.xy());
        tex_io.write_output::<RayTracingDebugOutput>(
          launch_id.xy(),
          (prev.x(), prev.y() + val(100. / 255.), prev.z(), val(1.)).into(),
        );
      }),
  );
  let miss = rtx_pipeline_desc.register_ray_miss::<RayCustomPayload>(
    shader_base_builder
      .create_miss_hit_shader_base::<RayCustomPayload>()
      .inject_ctx(texture_io_system.clone())
      .map(|_, ctx| {
        let miss_ctx = ctx.miss_hit_ctx().unwrap();
        let launch_id = miss_ctx.launch_id();

        let tex_io = ctx.registry.get_mut::<FrameOutputInvocation>().unwrap();
        let prev = tex_io.read_output::<RayTracingDebugOutput>(launch_id.xy());
        tex_io.write_output::<RayTracingDebugOutput>(
          launch_id.xy(),
          (prev.x() + val(100. / 255.), prev.y(), prev.z(), val(1.)).into(),
        );
      }),
  );

  let mesh_count = 1;
  let ray_type_count = 1;

  let rtx_pipeline = rtx_device.create_raytracing_pipeline(rtx_pipeline_desc);

  let mut sbt = rtx_device.create_sbt(mesh_count, ray_type_count);
  sbt.config_ray_generation(ray_gen);
  sbt.config_missing(0, miss);
  sbt.config_hit_group(
    0,
    0,
    HitGroupShaderRecord {
      closest_hit: Some(closest_hit),
      any_hit: None,
      intersection: None,
    },
  );

  let mut rtx_encoder = system.create_raytracing_encoder();

  rtx_encoder.set_pipeline(rtx_pipeline.as_ref());
  rtx_encoder.trace_ray((canvas_size, canvas_size, 1), sbt.as_ref());
  drop(rtx_encoder);

  let view = texture_io_system.take_output_target::<RayTracingDebugOutput>();

  let buffer = {
    let mut encoder = gpu.device.create_encoder();
    let texture = view.resource.clone().try_into();
    let buffer = encoder.read_texture_2d(
      &gpu.device,
      &texture.unwrap(),
      ReadRange {
        size: Size::from_u32_pair_min_one((canvas_size, canvas_size)),
        offset_x: 0,
        offset_y: 0,
      },
    );
    gpu.submit_encoder(encoder);
    buffer.await.unwrap()
  };

  let buffer = buffer.read_raw();
  let mut write_buffer = format!("P3\n{} {}\n255\n", canvas_size, canvas_size);
  buffer
    .chunks_exact(canvas_size as usize * 4)
    .for_each(|line| {
      line.chunks_exact(4).for_each(|pixel| {
        let (r, g, b, _a) = (pixel[0], pixel[1], pixel[2], pixel[3]);
        write_buffer.push_str(&format!("{r} {g} {b} "));
      });
      write_buffer.push('\n');
    });
  std::fs::write("trace.pbm", write_buffer).unwrap();
}
