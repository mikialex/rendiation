#[pollster::test]
async fn test_wavefront_compute() {
  use rendiation_texture_core::Size;

  let canvas_size = 64;

  use crate::*;
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();

  let mut texture_io_system = RayTracingTextureIO::default();

  pub struct RayTracingDebugOutput;
  impl RayTracingOutputTargetSemantic for RayTracingDebugOutput {}

  let debug_output = GPUTexture::create(
    TextureDescriptor {
      label: "tracing-debug".into(),
      size: Size::from_u32_pair_min_one((canvas_size, canvas_size)).into_gpu_size(),
      mip_level_count: 1,
      sample_count: 1,
      dimension: TextureDimension::D2,
      format: TextureFormat::Rgba8Unorm,
      view_formats: &[],
      usage: TextureUsages::all(),
    },
    &gpu.device,
  );
  let debug_output = GPU2DTexture::try_from(debug_output).unwrap();
  let debug_output_view = debug_output.create_default_view();
  let debug_output_view = GPU2DTextureView::try_from(debug_output_view).unwrap();

  texture_io_system.install_output_target::<RayTracingDebugOutput>(debug_output_view);

  let system = GPUWaveFrontComputeRaytracingSystem::new(&gpu);
  let as_sys = system.create_acceleration_structure_system();

  use crate::GPURaytracingSystem;
  init_default_acceleration_structure(as_sys.as_ref());

  let rtx_device = system.create_raytracing_device();

  let mut rtx_pipeline_desc = GPURaytracingPipelineDescriptor::default();

  let ray_gen_shader = WaveFrontTracingBaseProvider::create_ray_gen_shader_base()
    .inject_ctx(texture_io_system.clone())
    .then_trace(
      // (&T, &mut TracingCtx) -> (Node<bool>, ShaderRayTraceCall, Node<P>)
      |_, ctx| {
        let launch_info = ctx.registry.get_mut::<RayLaunchRawInfo>().unwrap();
        let launch_id = launch_info.launch_id();
        let launch_size = launch_info.launch_size();

        // let tex_io = ctx.registry.get_mut::<FrameOutputInvocation>().unwrap();
        // tex_io.write_output::<RayTracingDebugOutput>(
        //   val(vec2(1, 0)),
        //   // (color.into_f32(), val(vec3(1., 1., 1.))).into(),
        //   val(vec4(10., 1., 0.1, 1.)),
        // );

        const ORIGIN: Vec3<f32> = vec3(0., 0., -1.);
        let x =
          (launch_id.x().into_f32() + val(0.5)) / launch_size.x().into_f32() * val(2.) - val(1.);
        let y =
          val(1.) - (launch_id.y().into_f32() + val(0.5)) / launch_size.y().into_f32() * val(2.);
        let target: Node<Vec3<f32>> = (x, y, val(-1.)).into(); // fov = 90 deg
        let dir = (target - val(ORIGIN)).normalize();

        let trace_call = ShaderRayTraceCall {
          launch_id,
          launch_size,
          tlas_idx: val(0), // todo
          ray_flags: val(0),
          cull_mask: val(0xff),
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
    .map(|(_, payload), ctx| {
      let launch_info = ctx.registry.get_mut::<RayLaunchRawInfo>().unwrap();
      let launch_id = launch_info.launch_id();
      let payload: Node<RayCustomPayload> = payload;
      let color = payload.expand().color;
      let tex_io = ctx.registry.get_mut::<FrameOutputInvocation>().unwrap();
      tex_io.write_output::<RayTracingDebugOutput>(
        launch_id.xy(),
        (color.into_f32(), val(vec3(1., 1., 1.))).into(),
        // val(vec4(10., 1., 0.1, 1.)),
      );
    });

  #[derive(Copy, Clone, Debug, Default, ShaderStruct)]
  pub struct RayCustomPayload {
    pub color: u32,
  }

  let ray_gen = rtx_pipeline_desc.register_ray_gen::<RayCustomPayload>(ray_gen_shader);
  let closest_hit = rtx_pipeline_desc.register_ray_closest_hit::<RayCustomPayload>(
    WaveFrontTracingBaseProvider::create_closest_hit_shader_base::<RayCustomPayload>(),
  );
  let miss = rtx_pipeline_desc.register_ray_miss::<RayCustomPayload>(
    WaveFrontTracingBaseProvider::create_miss_hit_shader_base::<RayCustomPayload>(),
  );

  let mesh_count = 1;
  let ray_type_count = 1;

  let rtx_pipeline = rtx_device.create_raytracing_pipeline(&rtx_pipeline_desc);

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

  let _view = texture_io_system.take_output_target::<RayTracingDebugOutput>();

  let buffer = {
    let mut encoder = gpu.device.create_encoder();
    let buffer = encoder.read_texture_2d(
      &gpu.device,
      &debug_output,
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
  let mut write_buffer = format!("P6\n{} {}\n255\n", canvas_size, canvas_size).into_bytes();
  buffer.chunks_exact(4).for_each(|chunk| {
    let (r, g, b, _a) = (chunk[0], chunk[1], chunk[2], chunk[3]);
    if r > 0 || g > 0 || b > 0 {
      println!("!!");
    }
    write_buffer.extend_from_slice(&[r, g, b]);
  });
  std::fs::write("trace.pbm", write_buffer).unwrap();
}
