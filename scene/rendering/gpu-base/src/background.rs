use fast_hash_collection::FastHashMap;
use rendiation_shader_library::transform_dir_fn;

use crate::*;

pub fn use_background(cx: &mut QueryGPUHookCx) -> Option<SceneBackgroundRenderer> {
  let (env_background_map_gpu, _) = use_gpu_texture_cubes(cx, false);

  let env_background_intensity_uniform = cx.use_uniform_buffers();

  cx.use_changes::<SceneHDRxEnvBackgroundInfo>()
    .filter_map_changes(|v| {
      v.map(|v| IblShaderInfoForBackground {
        transform: v.transform,
        intensity: v.intensity,
        ..Default::default()
      })
    })
    .update_uniforms(&env_background_intensity_uniform, 0, cx.gpu);

  let solid_background_color_uniform = cx.use_uniform_buffers();

  cx.use_changes::<SceneSolidBackground>()
    .map_changes(|v| {
      v.map(srgb3_to_linear3)
        .unwrap_or(Vec3::splat(0.))
        .expand_with_one()
    })
    .update_uniforms(&solid_background_color_uniform, 0, cx.gpu);

  let gradient_background_uniform = cx.use_uniform_buffers();

  cx.use_changes::<SceneGradientBackgroundInfo>()
    .filter_map_changes(|v| {
      v.map(|param| {
        let mut color_and_stops: Vec<_> = param
          .color_and_stops
          .iter()
          .map(|v| srgb4_to_linear4(*v))
          .collect();

        color_and_stops.sort_by(|a, b| a.w.total_cmp(&b.w));

        let color_and_stops = Shader140Array::from_slice_clamp_or_default(&color_and_stops);
        if param.color_and_stops.len() > MAX_GRADIENT_COLOR_STOPS {
          log::warn!(
            "gradient color stops more than {} will be clamped",
            MAX_GRADIENT_COLOR_STOPS
          );
        }
        let color_and_stops_len = param.color_and_stops.len().min(MAX_GRADIENT_COLOR_STOPS) as u32;
        GradientBackgroundUniform {
          transform: param.transform,
          color_and_stops,
          color_and_stops_len,
          ..Default::default()
        }
      })
    })
    .update_uniforms(&gradient_background_uniform, 0, cx.gpu);

  cx.when_render(|| SceneBackgroundRenderer {
    solid_background: read_global_db_component(),
    env_background_map: read_global_db_foreign_key(),
    env_background_map_gpu: env_background_map_gpu.make_read_holder(),
    env_background_param: env_background_intensity_uniform.make_read_holder(),
    solid_background_uniform: solid_background_color_uniform.make_read_holder(),
    gradient_background_uniform: gradient_background_uniform.make_read_holder(),
    gradient_background: read_global_db_component(),
  })
}

pub struct SceneBackgroundRenderer {
  pub solid_background: ComponentReadView<SceneSolidBackground>,
  pub env_background_map: ForeignKeyReadView<SceneHDRxEnvBackgroundCubeMap>,
  pub env_background_map_gpu: LockReadGuardHolder<FastHashMap<RawEntityHandle, GPUCubeTextureView>>,
  pub env_background_param:
    LockReadGuardHolder<UniformBufferCollectionRaw<u32, IblShaderInfoForBackground>>,
  pub solid_background_uniform: LockReadGuardHolder<UniformBufferCollectionRaw<u32, Vec4<f32>>>,
  pub gradient_background: ComponentReadView<SceneGradientBackgroundInfo>,
  pub gradient_background_uniform:
    LockReadGuardHolder<UniformBufferCollectionRaw<u32, GradientBackgroundUniform>>,
}

impl SceneBackgroundRenderer {
  pub fn init_clear(
    &self,
    scene: EntityHandle<SceneEntity>,
    reversed_depth: bool,
  ) -> (Operations<rendiation_webgpu::Color>, Operations<f32>) {
    let color = self.solid_background.get_value(scene).unwrap();
    let color = color.unwrap_or(Vec3::splat(0.9));
    let color = rendiation_webgpu::Color {
      r: color.x as f64,
      g: color.y as f64,
      b: color.z as f64,
      a: 1.,
    };
    (
      clear_and_store(color),
      clear_and_store(if reversed_depth { 0. } else { 1. }),
    )
  }

  pub fn draw<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    camera: &'a dyn RenderComponent,
    tonemap: &'a dyn RenderComponent,
  ) -> impl PassContent + 'a {
    if let Some(env) = self.env_background_map.get(scene) {
      BackGroundDrawPassContent::CubeEnv(
        CubeEnvComponent {
          map: self
            .env_background_map_gpu
            .access(&env.into_raw())
            .unwrap()
            .clone(),
          param: self
            .env_background_param
            .access(&scene.alloc_index())
            .unwrap(),
          camera,
          tonemap,
        }
        .draw_quad(),
      )
    } else if let Some(Some(_)) = self.gradient_background.get(scene) {
      BackGroundDrawPassContent::Gradient(
        GradientBackgroundComponent {
          params: self
            .gradient_background_uniform
            .access(&scene.alloc_index())
            .unwrap(),
          camera,
        }
        .draw_quad(),
      )
    } else {
      BackGroundDrawPassContent::Solid
    }
  }
}

enum BackGroundDrawPassContent<'a> {
  Solid,
  Gradient(QuadDraw<GradientBackgroundComponent<'a>>),
  CubeEnv(QuadDraw<CubeEnvComponent<'a>>),
}

impl PassContent for BackGroundDrawPassContent<'_> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    match self {
      BackGroundDrawPassContent::Solid => {}
      BackGroundDrawPassContent::CubeEnv(cube) => cube.render(pass),
      BackGroundDrawPassContent::Gradient(gradient) => gradient.render(pass),
    }
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, PartialEq, Debug)]
pub struct IblShaderInfoForBackground {
  pub transform: Mat4<f32>,
  pub intensity: f32,
}

struct CubeEnvComponent<'a> {
  map: GPUCubeTextureView,
  param: UniformBufferDataView<IblShaderInfoForBackground>,
  camera: &'a dyn RenderComponent,
  tonemap: &'a dyn RenderComponent,
}

impl ShaderHashProvider for CubeEnvComponent<'_> {
  shader_hash_type_id! {CubeEnvComponent<'static>}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.camera.hash_pipeline_with_type_info(hasher);
    self.tonemap.hash_pipeline_with_type_info(hasher);
  }
}
impl ShaderPassBuilder for CubeEnvComponent<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.camera.setup_pass(ctx);
    ctx.binding.bind(&self.map);
    ctx.bind_immediate_sampler(&TextureSampler::default().with_double_linear().into_gpu());
    ctx.binding.bind(&self.param);
    self.tonemap.post_setup_pass(ctx);
  }
}

only_vertex!(EnvSampleDirectionVertex, Vec3<f32>);
only_fragment!(EnvSampleDirectionFrag, Vec3<f32>);

fn interpolate_direction(builder: &mut ShaderRenderPipelineBuilder) {
  builder.vertex(|builder, _| {
    let clip = builder.query::<ClipPosition>();
    let proj_inv = builder.query::<CameraProjectionInverseMatrix>();
    // camera view should be orthonormal
    let camera_rotation_only = builder
      .query::<CameraWorldNoneTranslationMatrix>()
      .shrink_to_3();
    let unprojected = proj_inv * clip;
    builder.register::<EnvSampleDirectionVertex>(camera_rotation_only * unprojected.xyz());
  });
}

impl GraphicsShaderProvider for CubeEnvComponent<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.camera.build(builder);
    interpolate_direction(builder);

    builder.fragment(|builder, binding| {
      let direction = builder
        .query_or_interpolate_by::<EnvSampleDirectionFrag, EnvSampleDirectionVertex>()
        .normalize();

      let cube = binding.bind_by(&self.map);
      let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);
      let params = binding.bind_by(&self.param).load().expand();
      let direction = transform_dir_fn(params.transform, direction);
      let result = cube.sample_zero_level(sampler, direction).xyz();

      builder.register::<HDRLightResult>(result * params.intensity);
    });

    self.tonemap.post_build(builder);

    builder.fragment(|builder, _| {
      let ldr = builder.query::<LDRLightResult>();
      let ldr: Node<Vec4<_>> = (ldr, val(1.)).into();
      builder.store_fragment_out(0, ldr);
    });

    mask_out_none_color_write(builder);
  }
}

pub const MAX_GRADIENT_COLOR_STOPS: usize = 8;

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct, Debug, PartialEq)]
pub struct GradientBackgroundUniform {
  pub transform: Mat4<f32>,
  pub color_and_stops: Shader140Array<Vec4<f32>, MAX_GRADIENT_COLOR_STOPS>,
  pub color_and_stops_len: u32,
}

struct GradientBackgroundComponent<'a> {
  params: UniformBufferDataView<GradientBackgroundUniform>,
  camera: &'a dyn RenderComponent,
}

impl ShaderHashProvider for GradientBackgroundComponent<'_> {
  shader_hash_type_id! {CubeEnvComponent<'static>}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.camera.hash_pipeline_with_type_info(hasher);
  }
}

impl GraphicsShaderProvider for GradientBackgroundComponent<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.camera.build(builder);
    interpolate_direction(builder);

    builder.fragment(|builder, binding| {
      let direction = builder
        .query_or_interpolate_by::<EnvSampleDirectionFrag, EnvSampleDirectionVertex>()
        .normalize();

      let params = binding.bind_by(&self.params);
      let direction = transform_dir_fn(params.transform().load(), direction);
      let v = direction_to_uv_fn(direction).y();

      let color = interpolate_gradient(
        v,
        params.color_and_stops(),
        params.color_and_stops_len().load(),
      );
      let color: Node<Vec4<f32>> = (color, val(1.)).into();

      builder.store_fragment_out(0, color);
    });

    mask_out_none_color_write(builder);
  }
}

fn interpolate_gradient(
  v: Node<f32>,
  stops: ShaderReadonlyPtrOf<[Vec4<f32>; MAX_GRADIENT_COLOR_STOPS]>,
  count: Node<u32>,
) -> Node<Vec3<f32>> {
  let pivot_start = stops.index(0).load();
  let pivot_end = stops.index(count - val(1)).load();

  let color = pivot_start.xyz().make_local_var();

  if_by(v.less_than(pivot_start.w()), || {
    color.store(pivot_start.xyz());
  });

  ForRange::ranged(vec2_node((val(0), count))).for_each(|i, cx| {
    let curr = stops.index(i).load();
    let next = stops.index(i + val(1)).load();

    if_by(
      v.greater_equal_than(curr.w()).and(v.less_than(next.w())),
      || {
        let distance = next.w() - curr.w();
        let t = (v - curr.w()) / distance;
        let c = t.mix(curr.xyz(), next.xyz());
        color.store(c);
        cx.do_break();
      },
    );
  });

  if_by(v.greater_than(pivot_end.w()), || {
    color.store(pivot_end.xyz());
  });

  color.load()
}

impl ShaderPassBuilder for GradientBackgroundComponent<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.camera.setup_pass(ctx);
    ctx.binding.bind(&self.params);
  }
}

#[shader_fn]
fn direction_to_uv(dir: Node<Vec3<f32>>) -> Node<Vec2<f32>> {
  let pi = val(f32::PI());

  let theta = dir.z().atan2(dir.x());
  let phi = dir.y().acos();
  let s = theta / (val(2.0) * pi);
  let t = phi / pi;
  (s + val(0.5), t).into()
}

/// the write target may contains other channel for example entity id, which
/// should not be written. we also check if platform support this case(webgl)
fn mask_out_none_color_write(builder: &mut ShaderRenderPipelineBuilder) {
  if !builder
    .info
    .downgrade_info
    .flags
    .contains(DownlevelFlags::INDEPENDENT_BLEND)
  {
    return;
  }

  builder.fragment(|builder, _| {
    builder
      .frag_output
      .iter_mut()
      .enumerate()
      .for_each(|(i, p)| {
        if i != 0 {
          p.states.write_mask = ColorWrites::empty();
        }
      });
  });
}
