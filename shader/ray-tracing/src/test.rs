#[pollster::test]
async fn test_wavefront_compute() {
  use rendiation_texture_core::Size;

  use crate::*;
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();

  let mut texture_io_system = RayTracingTextureIO::default();

  pub struct RayTracingDebugOutput;
  impl RayTracingOutputTargetSemantic for RayTracingDebugOutput {}

  let debug_output = GPUTexture::create(
    TextureDescriptor {
      label: "tracing-debug".into(),
      size: Size::from_u32_pair_min_one((1, 1)).into_gpu_size(),
      mip_level_count: 1,
      sample_count: 1,
      dimension: TextureDimension::D2,
      format: TextureFormat::Rgba8Unorm,
      view_formats: &[],
      usage: TextureUsages::all(),
    },
    &gpu.device,
  );
  let debug_output = GPU2DTexture::try_from(debug_output)
    .unwrap()
    .create_default_view();
  let debug_output = GPU2DTextureView::try_from(debug_output).unwrap();

  texture_io_system.install_output_target::<RayTracingDebugOutput>(debug_output);

  let system = GPUWaveFrontComputeRaytracingSystem::new(&gpu);
  let as_sys = system.create_acceleration_structure_system();

  use crate::GPURaytracingSystem;
  init_default_acceleration_structure(as_sys.as_ref());

  let rtx_device = system.create_raytracing_device();

  let mut rtx_pipeline_desc = GPURaytracingPipelineDescriptor::default();

  // todo, remove ray gen payload
  let ray_gen_shader = WaveFrontTracingBaseProvider::create_ray_gen_shader_base()
    .inject_ctx(texture_io_system.clone())
    .then_trace(
      // (&T, &mut TracingCtx) -> (Node<bool>, ShaderRayTraceCall, Node<P>)
      |_, _ctx| {
        let trace_call = ShaderRayTraceCall {
          tlas_idx: val(0), // todo
          ray_flags: val(0),
          cull_mask: val(0xff),
          sbt_ray_config: RaySBTConfig {
            offset: val(0),
            stride: val(0),
          },
          miss_index: val(0),
          // todo ray from x,y
          ray: ShaderRay {
            origin: val(vec3(0., 0., 1.)),
            direction: val(vec3(0., 0., -1.)),
          },
          range: ShaderRayRange {
            min: val(0.1),
            max: val(100.),
          },
          payload: val(0),
        };

        let ray_payload = ENode::<RayCustomPayload> { color: val(0) }.construct();

        (val(true), trace_call, ray_payload)
      },
    )
    .map(|(_, _payload), ctx| {
      let tex_io = ctx.registry.get_mut::<FrameOutputInvocation>().unwrap();
      tex_io.write_output::<RayTracingDebugOutput>(val(Vec2::zero()), val(Vec4::zero()));
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

  let canvas_size = 1;

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

  texture_io_system.take_output_target::<RayTracingDebugOutput>();
}
