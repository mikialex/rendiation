use std::hash::Hash;

use super::*;

pub fn build_ray_miss_shader(
  trace_base_builder: &TraceFutureBaseBuilder,
  ctx: PTRayMissCtx,
) -> impl TraceOperator<()> + 'static {
  trace_base_builder
    .create_miss_hit_shader_base::<CorePathPayload>()
    .inject_ctx(ctx)
    .map(|_, cx| {
      let miss_cx = cx.expect_miss_hit_ctx();
      let pt_cx = cx.expect_custom_cx::<PTRayMissCtxInvocation>();

      let radiance = pt_cx.sample_radiance(miss_cx.world_ray().direction);

      cx.expect_payload::<CorePathPayload>()
        .sampled_radiance()
        .store(radiance);

      cx.expect_payload::<CorePathPayload>()
        .missed()
        .store(val(true).into_big_bool());
    })
}

#[derive(Clone)]
pub enum PTRayMissCtx {
  EnvCube {
    map: GPUCubeTextureView,
    intensity: UniformBufferDataView<Vec4<f32>>,
  },
  Solid {
    color: UniformBufferDataView<Vec4<f32>>,
  },
  #[allow(dead_code)]
  Test,
}

impl PTRayMissCtx {
  pub fn new(renderer: &SceneBackgroundRenderer, scene: EntityHandle<SceneEntity>) -> Self {
    if let Some(env) = renderer.env_background_map.get(scene) {
      PTRayMissCtx::EnvCube {
        map: renderer
          .env_background_map_gpu
          .access(&env)
          .unwrap()
          .clone(),
        intensity: renderer.env_background_intensity.access(&scene).unwrap(),
      }
    } else {
      PTRayMissCtx::Solid { color: todo!() }
    }
  }
}

impl ShaderHashProvider for PTRayMissCtx {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    std::mem::discriminant(self).hash(hasher);
  }
}

impl RayTracingCustomCtxProvider for PTRayMissCtx {
  type Invocation = PTRayMissCtxInvocation;

  fn build_invocation(&self, cx: &mut ShaderBindGroupBuilder) -> Self::Invocation {
    match self {
      Self::EnvCube { map, intensity } => PTRayMissCtxInvocation::EnvCube {
        map: cx.bind_by(map),
        intensity: cx.bind_by(intensity),
        sampler: todo!(),
      },
      Self::Solid { color } => PTRayMissCtxInvocation::Solid(cx.bind_by(color)),
      Self::Test => PTRayMissCtxInvocation::Test,
    }
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    match self {
      Self::EnvCube { map, intensity } => {
        builder.bind(map);
        builder.bind(intensity);
      }
      Self::Solid { color } => {
        builder.bind(color);
      }
      Self::Test => {}
    }
  }
}

#[derive(Clone)]
pub enum PTRayMissCtxInvocation {
  Solid(ShaderReadonlyPtrOf<Vec4<f32>>),
  EnvCube {
    map: BindingNode<ShaderTextureCube>,
    intensity: ShaderReadonlyPtrOf<Vec4<f32>>,
    sampler: BindingNode<ShaderSampler>,
  },
  Test,
}

impl PTRayMissCtxInvocation {
  pub fn sample_radiance(&self, world_ray_direction: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
    match self {
      Self::Solid(color) => color.load().xyz(),
      Self::EnvCube {
        map,
        intensity,
        sampler,
      } => map.sample_zero_level(*sampler, world_ray_direction).xyz() * intensity.load().xyz(),
      Self::Test => world_ray_direction
        .y()
        .greater_than(0.)
        .select(Vec3::splat(0.7), Vec3::splat(0.3)),
    }
  }
}
