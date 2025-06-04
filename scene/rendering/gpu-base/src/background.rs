use fast_hash_collection::FastHashMap;

use crate::*;

pub fn use_background<'a>(cx: &'a mut QueryGPUHookCx<'a>) -> Option<SceneBackgroundRenderer<'a>> {
  let (cx, env_background_map_gpu) =
    cx.use_multi_updater_ref(|gpu| gpu_texture_cubes(gpu, FastHashMap::default()));

  let (cx, env_background_intensity_uniform) = cx
    .use_uniform_buffers_ref::<EntityHandle<SceneEntity>, Vec4<f32>>(|source, gpu| {
      source.with_source(
        global_watch()
          .watch::<SceneHDRxEnvBackgroundIntensity>()
          .collective_filter_map(|v| v.map(|intensity| Vec4::new(intensity, 0., 0., 0.)))
          .into_query_update_uniform(0, gpu),
      )
    });

  let (cx, solid_background_color_uniform) = cx
    .use_uniform_buffers_ref::<EntityHandle<SceneEntity>, Vec4<f32>>(|source, gpu| {
      source.with_source(
        global_watch()
          .watch::<SceneSolidBackground>()
          .collective_map(|v| {
            v.map(srgb3_to_linear3)
              .unwrap_or(Vec3::splat(0.))
              .expand_with_one()
          })
          .into_query_update_uniform(0, gpu),
      )
    });

  cx.when_create_impl(|| SceneBackgroundRenderer {
    solid_background: global_entity_component_of::<SceneSolidBackground>().read(),
    env_background_map: global_entity_component_of::<SceneHDRxEnvBackgroundCubeMap>()
      .read_foreign_key(),
    env_background_map_gpu: env_background_map_gpu.unwrap(),
    env_background_intensity: env_background_intensity_uniform.unwrap(),
    solid_background_uniform: solid_background_color_uniform.unwrap(),
  })
}

pub struct SceneBackgroundRenderer<'a> {
  pub solid_background: ComponentReadView<SceneSolidBackground>,
  pub env_background_map: ForeignKeyReadView<SceneHDRxEnvBackgroundCubeMap>,
  pub env_background_map_gpu:
    &'a FastHashMap<EntityHandle<SceneTextureCubeEntity>, GPUCubeTextureView>,
  pub env_background_intensity:
    &'a FastHashMap<EntityHandle<SceneEntity>, UniformBufferDataView<Vec4<f32>>>,
  pub solid_background_uniform:
    &'a FastHashMap<EntityHandle<SceneEntity>, UniformBufferDataView<Vec4<f32>>>,
}

impl<'a> SceneBackgroundRenderer<'a> {
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

  pub fn draw(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    camera: Box<dyn RenderComponent + 'a>,
    tonemap: &'a dyn RenderComponent,
  ) -> impl PassContent + 'a {
    if let Some(env) = self.env_background_map.get(scene) {
      BackGroundDrawPassContent::CubeEnv(
        CubeEnvComponent {
          map: self.env_background_map_gpu.access(&env).unwrap().clone(),
          intensity: self.env_background_intensity.access(&scene).unwrap(),
          camera,
          tonemap,
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
  CubeEnv(QuadDraw<CubeEnvComponent<'a>>),
}

impl PassContent for BackGroundDrawPassContent<'_> {
  fn render(&mut self, pass: &mut FrameRenderPass) {
    match self {
      BackGroundDrawPassContent::Solid => {} // implemented in pass clear, nothing to do here
      BackGroundDrawPassContent::CubeEnv(cube) => cube.render(pass),
    }
  }
}

struct CubeEnvComponent<'a> {
  map: GPUCubeTextureView,
  intensity: UniformBufferDataView<Vec4<f32>>,
  camera: Box<dyn RenderComponent + 'a>,
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
    ctx.binding.bind(&self.intensity);
    self.tonemap.post_setup_pass(ctx);
  }
}

only_vertex!(EnvSampleDirectionVertex, Vec3<f32>);
only_fragment!(EnvSampleDirectionFrag, Vec3<f32>);

impl GraphicsShaderProvider for CubeEnvComponent<'_> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.camera.build(builder);

    builder.vertex(|builder, _| {
      let clip = builder.query::<ClipPosition>();
      let proj_inv = builder.query::<CameraProjectionInverseMatrix>();
      // camera view should be orthonormal
      let view_inverse = builder
        .query::<CameraViewMatrix>()
        .shrink_to_3()
        .transpose();
      let unprojected = proj_inv * clip;
      builder.register::<EnvSampleDirectionVertex>(view_inverse * unprojected.xyz());
    });

    builder.fragment(|builder, binding| {
      let direction =
        builder.query_or_interpolate_by::<EnvSampleDirectionFrag, EnvSampleDirectionVertex>();

      let cube = binding.bind_by(&self.map);
      let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);
      let intensity = binding.bind_by(&self.intensity).load().x();
      let result = cube.sample_zero_level(sampler, direction).xyz();

      builder.register::<HDRLightResult>(result * intensity);
    });

    self.tonemap.post_build(builder);

    builder.fragment(|builder, _| {
      let ldr = builder.query::<LDRLightResult>();
      let ldr: Node<Vec4<_>> = (ldr, val(1.)).into();
      builder.store_fragment_out(0, ldr);
    })
  }
}
