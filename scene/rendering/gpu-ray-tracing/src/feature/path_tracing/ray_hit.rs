use super::*;

pub fn build_ray_hit_shader(
  trace_base_builder: &TraceFutureBaseBuilder,
  ctx: PTRayClosestCtx,
) -> impl TraceOperator<()> + 'static {
  trace_base_builder
    .create_closest_hit_shader_base::<CorePathPayload>()
    .inject_ctx(ctx)
    .map(|_, ctx| {
      let pt_cx = ctx.expect_custom_cx::<PTClosestCtxInvocation>();
      let closest_hit_ctx = ctx.expect_closest_hit_ctx();

      let (shading_normal, geometry_normal, uv) =
        pt_cx.bindless_mesh.get_world_normal_and_uv(closest_hit_ctx);
      let sm_id = closest_hit_ctx.instance_custom_id();
      let in_dir = closest_hit_ctx.world_ray().direction;

      let seed = closest_hit_ctx.launch_id().xy();
      let seed = (
        seed.x(),
        seed.y(),
        pt_cx.config.current_sample_count().load(),
      );
      let sampler = &PCGRandomSampler::new(xxhash32(seed.into()));

      let RTSurfaceInteraction {
        sampling_dir,
        brdf,
        pdf,
        surface_radiance,
      } = pt_cx
        .surface
        .importance_sampling_brdf(sm_id, in_dir, shading_normal, uv, sampler);

      let out_ray_origin = closest_hit_ctx.hit_world_position();
      let out_ray_origin = offset_ray_hit_fn(out_ray_origin, geometry_normal);

      let payload = ctx.expect_payload::<CorePathPayload>();
      payload.next_ray_origin().store(out_ray_origin);
      payload.next_ray_dir().store(sampling_dir);
      payload.normal().store(shading_normal);
      payload.brdf().store(brdf);
      payload.pdf().store(pdf);
      payload.sampled_radiance().store(surface_radiance);
      payload.missed().store(val(false).into_big_bool());
      //
    })
}

#[derive(Clone)]
pub struct PTRayClosestCtx {
  pub bindless_mesh: BindlessMeshDispatcher,
  pub surface: Box<dyn DevicePathTracingSurface>,
  pub config: UniformBufferDataView<PTConfig>,
}

impl ShaderHashProvider for PTRayClosestCtx {
  shader_hash_type_id! {}
}

impl RayTracingCustomCtxProvider for PTRayClosestCtx {
  type Invocation = PTClosestCtxInvocation;

  fn build_invocation(&self, cx: &mut ShaderBindGroupBuilder) -> Self::Invocation {
    PTClosestCtxInvocation {
      bindless_mesh: self.bindless_mesh.build_bindless_mesh_rtx_access(cx),
      surface: self.surface.build(cx),
      config: cx.bind_by(&self.config),
    }
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    self.bindless_mesh.bind_bindless_mesh_rtx_access(builder);
    self.surface.bind(builder);
    builder.bind(&self.config);
  }
}

#[derive(Clone)]
pub struct PTClosestCtxInvocation {
  bindless_mesh: BindlessMeshRtxAccessInvocation,
  surface: Box<dyn DevicePathTracingSurfaceInvocation>,
  config: ShaderReadonlyPtrOf<PTConfig>,
}
