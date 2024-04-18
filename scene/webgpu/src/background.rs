use crate::*;

pub trait WebGPUBackground: 'static + SceneRenderable {
  fn require_pass_clear(&self) -> Option<Color>;
}
define_dyn_trait_downcaster_static!(WebGPUBackground);

impl WebGPUBackground for SolidBackground {
  fn require_pass_clear(&self) -> Option<Color> {
    Color {
      r: self.intensity.r() as f64,
      g: self.intensity.g() as f64,
      b: self.intensity.b() as f64,
      a: 1.,
    }
    .into()
  }
}

impl SceneRenderable for SolidBackground {
  fn render<'a>(
    &self,
    _pass: &mut FrameRenderPass,
    _dispatcher: &dyn RenderComponentAny,
    _camera: &SceneCamera,
    _scene: &SceneRenderResourceGroup,
  ) {
  }
}

impl WebGPUBackground for EnvMapBackground {
  fn require_pass_clear(&self) -> Option<Color> {
    None
  }
}

impl SceneRenderable for EnvMapBackground {
  fn render<'a>(
    &self,
    pass: &mut FrameRenderPass,
    base: &dyn RenderComponentAny,
    camera: &SceneCamera,
    scene: &SceneRenderResourceGroup,
  ) {
    let (_, texture) = scene
      .resources
      .bindable_ctx
      .get_or_create_reactive_gpu_texture_cube(&self.texture);

    let cameras = scene.scene_resources.cameras.read().unwrap();
    let camera_gpu = cameras.get_camera_gpu(camera).unwrap();

    // should we cache it?
    let content = EnvMapBackgroundGPU { texture };
    let content = ShadingBackgroundTask {
      content,
      camera_gpu,
    };

    let components: [&dyn RenderComponentAny; 3] = [&base, &FullScreenQuad::default(), &content];

    RenderEmitter::new(components.as_slice()).render(&mut pass.ctx, QUAD_DRAW_CMD);
  }
}

struct EnvMapBackgroundGPU {
  texture: GPUCubeTextureView,
}

impl ShaderHashProvider for EnvMapBackgroundGPU {}
impl ShadingBackground for EnvMapBackgroundGPU {
  fn shading(
    &self,
    builder: &mut ShaderFragmentBuilderView,
    binding: &mut ShaderBindGroupDirectBuilder,
    direction: Node<Vec3<f32>>,
  ) -> Result<(), ShaderBuildError> {
    let cube = binding.bind_by(&self.texture);
    let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);
    cube.sample(sampler, direction);
    builder.register::<DefaultDisplay>(cube.sample(sampler, direction));
    Ok(())
  }
}

impl ShaderPassBuilder for EnvMapBackgroundGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.texture);
    ctx.bind_immediate_sampler(&TextureSampler::default().with_double_linear().into_gpu());
  }
}

struct ShadingBackgroundTask<'a, T> {
  content: T,
  camera_gpu: &'a CameraGPU,
}

pub trait ShadingBackground {
  fn shading(
    &self,
    builder: &mut ShaderFragmentBuilderView,
    binding: &mut ShaderBindGroupDirectBuilder,
    direction: Node<Vec3<f32>>,
  ) -> Result<(), ShaderBuildError>;
}

impl<'a, T: ShaderPassBuilder> ShaderPassBuilder for ShadingBackgroundTask<'a, T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.camera_gpu.setup_pass(ctx);
    self.content.setup_pass(ctx)
  }
}

impl<'a, T: ShaderHashProvider> ShaderHashProvider for ShadingBackgroundTask<'a, T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.content.hash_pipeline(hasher)
  }
}
impl<'a, T: ShaderHashProvider + Any> ShaderHashProviderAny for ShadingBackgroundTask<'a, T> {
  fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
    struct Mark;
    Mark.type_id().hash(hasher);
    self.content.type_id().hash(hasher);
  }
}

both!(CameraWorldDirection, Vec3<f32>);

impl<'a, T: ShadingBackground> GraphicsShaderProvider for ShadingBackgroundTask<'a, T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    self.camera_gpu.inject_uniforms(builder);
    builder.vertex(|builder, _| {
      let vertex_index = builder.query::<VertexIndex>()?;
      let projection_inv = builder.query::<CameraProjectionInverseMatrix>()?;
      let view = builder.query::<CameraViewMatrix>()?;

      let vert = generate_quad(vertex_index, 1.).expand();
      builder.register::<ClipPosition>(vert.position);

      let model_view_inv = (view).transpose(); // we assume these are orthogonal
      let unprojected = projection_inv * vert.position;
      let direction = (model_view_inv * unprojected).xyz();

      builder.set_vertex_out::<CameraWorldDirection>(direction);
      Ok(())
    })?;

    builder.fragment(|builder, binding| {
      let direction = builder.query::<CameraWorldDirection>()?;
      let direction = direction * val(Vec3::new(-1., 1., 1.)); // left hand texture space
      self.content.shading(builder, binding, direction)?;
      Ok(())
    })
  }
}
