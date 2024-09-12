#[pollster::test]
async fn test_wavefront_compute() {
  use crate::*;
  //
  let (gpu, _) = GPU::new(Default::default()).await.unwrap();

  let system = GPUWaveFrontComputeRaytracingSystem::new(&gpu);

  let rtx_device = system.create_raytracing_device();

  let mut rtx_pipeline_desc = GPURaytracingPipelineDescriptor::default();

  let ray_gen = rtx_pipeline_desc.register_ray_gen::<u32>(TraceBase::<()>::default());
  let closest_hit = rtx_pipeline_desc.register_ray_closest_hit::<u32>(TraceBase::<()>::default());

  let mesh_count = 4;
  let ray_type_count = 1;

  let canvas_size = 10;

  let rtx_pipeline = rtx_device.create_raytracing_pipeline(&rtx_pipeline_desc);

  let mut sbt = rtx_device.create_sbt(mesh_count, ray_type_count);
  sbt.config_ray_generation(ray_gen);
  sbt.config_hit_group(
    0,
    0,
    HitGroupShaderRecord {
      closet_hit: Some(closest_hit),
      any_hit: None,
      intersection: None,
    },
  );

  let mut rtx_encoder = system.create_raytracing_encoder();

  rtx_encoder.set_pipeline(rtx_pipeline.as_ref());
  rtx_encoder.trace_ray((canvas_size, canvas_size, 1), sbt.as_ref());
}
