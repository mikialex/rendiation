use core::f32;

use rendiation_device_ray_tracing::RayFlagConfigRaw::RAY_FLAG_ACCEPT_FIRST_HIT_AND_END_SEARCH;

use super::*;

pub fn build_ray_hit_shader(
  trace_base_builder: &TraceFutureBaseBuilder,
  ctx: PTRayClosestCtx,
) -> impl TraceOperator<()> + 'static {
  trace_base_builder
    .create_closest_hit_shader_base::<CorePathPayload>()
    .inject_ctx(ctx)
    .then_trace(|_, ctx| {
      // do light sampling and test shadow ray
      let pt_cx = ctx.expect_custom_cx::<PTClosestCtxInvocation>();
      let closest_hit_ctx = ctx.expect_closest_hit_ctx();

      let sample_count = pt_cx.config.current_sample_count().load();
      let sampler =
        &PCGRandomSampler::from_ray_ctx_and_sample_index(closest_hit_ctx, sample_count * val(2));

      let origin = closest_hit_ctx.hit_world_position();

      let (light_sample, should_sample) = pt_cx.lighting.importance_sampling_light(origin, sampler);

      let (_, geometry_normal, _) = pt_cx.bindless_mesh.get_world_normal_and_uv(closest_hit_ctx);
      let out_ray_origin = offset_ray_hit_fn(origin, geometry_normal);

      let ray = ShaderRay {
        origin: out_ray_origin,
        direction: light_sample.sampling_dir,
      };

      let trace_call = ShaderRayTraceCall {
        tlas_idx: val(0),
        ray_flags: val(RAY_FLAG_ACCEPT_FIRST_HIT_AND_END_SEARCH as u32),
        cull_mask: val(u32::MAX),
        sbt_ray_config: PTRayType::ShadowTest.to_sbt_cfg(),
        miss_index: val(1),
        ray,
        range: ShaderRayRange {
          min: val(f32::EPSILON),
          max: light_sample.distance,
        },
      };

      let payload = ENode::<ShaderTestPayload> {
        radiance: should_sample.select(
          light_sample.radiance / light_sample.pdf.splat(),
          zeroed_val(),
        ),
        light_sample_dir: out_ray_origin,
      }
      .construct();

      (should_sample, trace_call, payload)
    })
    .map(|(_, light_sample_result), ctx| {
      let pt_cx = ctx.expect_custom_cx::<PTClosestCtxInvocation>();
      let closest_hit_ctx = ctx.expect_closest_hit_ctx();

      let (shading_normal, geometry_normal, uv) =
        pt_cx.bindless_mesh.get_world_normal_and_uv(closest_hit_ctx);
      let sm_id = closest_hit_ctx.instance_custom_id();
      let view_dir = -closest_hit_ctx.world_ray().direction;

      let sample_count = pt_cx.config.current_sample_count().load();
      let sampler =
        &PCGRandomSampler::from_ray_ctx_and_sample_index(closest_hit_ctx, sample_count * val(3));

      let surface = pt_cx
        .surface
        .construct_shading_point(sm_id, shading_normal, uv);

      let RTSurfaceInteraction {
        sampling_dir,
        brdf,
        pdf,
        surface_radiance,
      } = surface.importance_sampling_brdf(view_dir, sampler);

      let out_ray_origin = closest_hit_ctx.hit_world_position();
      let out_ray_origin = offset_ray_hit_fn(out_ray_origin, geometry_normal);

      let light_sample_result = light_sample_result.expand();
      let direct_light_sample_brdf =
        surface.eval_brdf(view_dir, light_sample_result.light_sample_dir);
      let light_dot = shading_normal
        .dot(light_sample_result.light_sample_dir)
        .abs();
      // note, the lighting pdf has already applied in radiance
      let sampled_radiance = light_dot * direct_light_sample_brdf * light_sample_result.radiance;

      let payload = ctx.expect_payload::<CorePathPayload>();
      payload.next_ray_origin().store(out_ray_origin);
      payload.next_ray_dir().store(sampling_dir);
      payload.normal().store(shading_normal);
      payload.brdf().store(brdf);
      payload.pdf().store(pdf);
      payload.sampled_radiance().store(sampled_radiance);
      payload.surface_radiance().store(surface_radiance);
      payload.missed().store(val(false).into_big_bool());
      //
    })
}

#[derive(Clone)]
pub struct PTRayClosestCtx {
  pub bindless_mesh: BindlessMeshDispatcher,
  pub surface: Box<dyn DevicePathTracingSurface>,
  pub lighting: Box<dyn DevicePathTracingLighting>,
  pub config: UniformBufferDataView<PTConfig>,
}

impl ShaderHashProvider for PTRayClosestCtx {
  shader_hash_type_id! {}

  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.bindless_mesh.hash_pipeline(hasher);
    self.surface.hash_pipeline_with_type_info(hasher);
    self.lighting.hash_pipeline_with_type_info(hasher);
  }
}

impl RayTracingCustomCtxProvider for PTRayClosestCtx {
  type Invocation = PTClosestCtxInvocation;

  fn build_invocation(&self, cx: &mut ShaderBindGroupBuilder) -> Self::Invocation {
    PTClosestCtxInvocation {
      bindless_mesh: self.bindless_mesh.build_bindless_mesh_rtx_access(cx),
      surface: self.surface.build(cx),
      lighting: self.lighting.build(cx),
      config: cx.bind_by(&self.config),
    }
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    self.bindless_mesh.bind_bindless_mesh_rtx_access(builder);
    self.surface.bind(builder);
    self.lighting.bind(builder);
    builder.bind(&self.config);
  }
}

#[derive(Clone)]
pub struct PTClosestCtxInvocation {
  bindless_mesh: BindlessMeshRtxAccessInvocation,
  surface: Box<dyn DevicePathTracingSurfaceInvocation>,
  lighting: Box<dyn DevicePathTracingLightingInvocation>,
  config: ShaderReadonlyPtrOf<PTConfig>,
}
