#[test]
fn test_wavefront_compute() {
  pollster::block_on(async {
    {
      use crate::*;
      //
      let (gpu, _) = GPU::new(Default::default()).await.unwrap();

      let system = GPUWaveFrontComputeRaytracingSystem::new(&gpu);
      let as_sys = system.create_acceleration_structure_system();

      use crate::GPURaytracingSystem;
      init_default_acceleration_structure(as_sys.as_ref());

      let rtx_device = system.create_raytracing_device();

      let mut rtx_pipeline_desc = GPURaytracingPipelineDescriptor::default();

      let ray_gen_shader = TraceBase::<()>::default().then_trace(
        // (&T, &mut TracingCtx) -> (Node<bool>, ShaderRayTraceCall, Node<P>)
        |_, _ctx| {
          (
            val(true),
            ShaderRayTraceCall {
              tlas_idx: val(0), // todo what is this?
              ray_flags: val(0),
              cull_mask: val(0xff),
              sbt_ray_config: RaySBTConfig {
                offset: val(0),
                stride: val(0),
              },
              miss_index: val(0),
              ray: ShaderRay {
                origin: val(vec3(0., 0., 1.)),
                direction: val(vec3(0., 0., -1.)),
              },
              range: ShaderRayRange {
                min: val(0.1),
                max: val(100.),
              },
              payload: val(1),
            },
            val(0u32),
          )
        },
      );
      let ray_gen_shader = ray_gen_shader.map(|_a, _b| ());

      let ray_gen = rtx_pipeline_desc.register_ray_gen::<u32>(ray_gen_shader);
      let closest_hit =
        rtx_pipeline_desc.register_ray_closest_hit::<u32>(
          WaveFrontTracingBaseProvider::closest_shader_base::<u32>(),
        );
      let miss = rtx_pipeline_desc
        .register_ray_miss::<u32>(WaveFrontTracingBaseProvider::missing_shader_base::<u32>());

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
    }
  })
}
