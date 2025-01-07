use crate::*;

#[derive(Default)]
pub struct SceneBackgroundRendererSource {
  env_background_intensity_uniform: UpdateResultToken,
  // todo
  // note, currently the cube map is standalone maintained, this is wasteful if user shared it elsewhere
  cube_map: UpdateResultToken,
}

impl RenderImplProvider<SceneBackgroundRenderer> for SceneBackgroundRendererSource {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.cube_map = source.register_multi_updater(gpu_texture_cubes(cx));
    let cx = cx.clone();
    let intensity = global_watch()
      .watch::<SceneHDRxEnvBackgroundIntensity>()
      .collective_filter_map(|v| v)
      .collective_execute_map_by(move || {
        let cx = cx.clone();
        move |_, intensity| create_uniform(Vec4::new(intensity, 0., 0., 0.), &cx.device)
      })
      .materialize_unordered();
    self.env_background_intensity_uniform = source.register_reactive_query(intensity);
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.env_background_intensity_uniform);
    source.deregister(&mut self.cube_map);
  }

  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> SceneBackgroundRenderer {
    SceneBackgroundRenderer {
      solid_background: global_entity_component_of::<SceneSolidBackground>().read(),
      env_background_map: global_entity_component_of::<SceneHDRxEnvBackgroundCubeMap>()
        .read_foreign_key(),
      env_background_map_gpu: res.take_multi_updater_updated(self.cube_map).unwrap(),
      env_background_intensity: res
        .take_reactive_query_updated(self.env_background_intensity_uniform)
        .unwrap(),
    }
  }
}

pub struct SceneBackgroundRenderer {
  solid_background: ComponentReadView<SceneSolidBackground>,
  env_background_map: ForeignKeyReadView<SceneHDRxEnvBackgroundCubeMap>,
  env_background_map_gpu:
    LockReadGuardHolder<CubeMapUpdateContainer<EntityHandle<SceneTextureCubeEntity>>>,
  env_background_intensity:
    BoxedDynQuery<EntityHandle<SceneEntity>, UniformBufferDataView<Vec4<f32>>>,
}

impl SceneBackgroundRenderer {
  pub fn init_clear(
    &self,
    scene: EntityHandle<SceneEntity>,
  ) -> (Operations<rendiation_webgpu::Color>, Operations<f32>) {
    let color = self.solid_background.get_value(scene).unwrap();
    let color = color.unwrap_or(Vec3::splat(0.9));
    let color = rendiation_webgpu::Color {
      r: color.x as f64,
      g: color.y as f64,
      b: color.z as f64,
      a: 1.,
    };
    (clear(color), clear(1.))
  }

  pub fn draw<'a>(
    &'a self,
    scene: EntityHandle<SceneEntity>,
    camera: Box<dyn RenderDependencyComponent + 'a>,
  ) -> impl PassContent + 'a {
    if let Some(env) = self.env_background_map.get(scene) {
      BackGroundDrawPassContent::CubeEnv(
        CubeEnvComponent {
          map: self.env_background_map_gpu.access(&env).unwrap().clone(),
          intensity: self.env_background_intensity.access(&scene).unwrap(),
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
  CubeEnv(QuadDraw<CubeEnvComponent<'a>>),
}

impl<'a> PassContent for BackGroundDrawPassContent<'a> {
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
  camera: Box<dyn RenderDependencyComponent + 'a>,
}

impl<'a> ShaderHashProvider for CubeEnvComponent<'a> {
  shader_hash_type_id! {CubeEnvComponent<'static>}
}
impl<'a> ShaderPassBuilder for CubeEnvComponent<'a> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.camera.setup_pass(ctx);
    ctx.binding.bind(&self.map);
    ctx.bind_immediate_sampler(&TextureSampler::default().into_gpu());
    ctx.binding.bind(&self.intensity);
  }
}

only_vertex!(EnvSampleDirectionVertex, Vec3<f32>);
only_fragment!(EnvSampleDirectionFrag, Vec3<f32>);

impl<'a> GraphicsShaderProvider for CubeEnvComponent<'a> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    self.camera.inject_shader_dependencies(builder);

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
      let _intensity = binding.bind_by(&self.intensity).load().x(); // todo tonemap
      let result = cube.sample_zero_level(sampler, direction);

      builder.store_fragment_out(0, result);
    });
  }
}
