use fast_hash_collection::FastHashMap;

use crate::*;

#[derive(Default)]
pub struct SceneBackgroundRendererSource {
  env_background_intensity_uniform: QueryToken,
  // todo
  // note, currently the cube map is standalone maintained, this is wasteful if user shared it elsewhere
  cube_map: QueryToken,
}

impl QueryBasedFeature<SceneBackgroundRenderer> for SceneBackgroundRendererSource {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, gpu: &GPU) {
    self.cube_map = qcx.register_multi_updater(gpu_texture_cubes(gpu, FastHashMap::default()));
    let gpu = gpu.clone();
    let intensity = global_watch()
      .watch::<SceneHDRxEnvBackgroundIntensity>()
      .collective_filter_map(|v| v)
      .collective_execute_map_by(move || {
        let gpu = gpu.clone();
        move |_, intensity| create_uniform(Vec4::new(intensity, 0., 0., 0.), &gpu.device)
      })
      .materialize_unordered();
    self.env_background_intensity_uniform = qcx.register_reactive_query(intensity);
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.env_background_intensity_uniform);
    qcx.deregister(&mut self.cube_map);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> SceneBackgroundRenderer {
    SceneBackgroundRenderer {
      solid_background: global_entity_component_of::<SceneSolidBackground>().read(),
      env_background_map: global_entity_component_of::<SceneHDRxEnvBackgroundCubeMap>()
        .read_foreign_key(),
      env_background_map_gpu: cx.take_multi_updater_updated(self.cube_map).unwrap(),
      env_background_intensity: cx
        .take_reactive_query_updated(self.env_background_intensity_uniform)
        .unwrap(),
    }
  }
}

pub struct SceneBackgroundRenderer {
  pub solid_background: ComponentReadView<SceneSolidBackground>,
  pub env_background_map: ForeignKeyReadView<SceneHDRxEnvBackgroundCubeMap>,
  pub env_background_map_gpu:
    LockReadGuardHolder<CubeMapUpdateContainer<EntityHandle<SceneTextureCubeEntity>>>,
  pub env_background_intensity:
    BoxedDynQuery<EntityHandle<SceneEntity>, UniformBufferDataView<Vec4<f32>>>,
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
    (clear(color), clear(if reversed_depth { 0. } else { 1. }))
  }

  pub fn draw<'a>(
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
