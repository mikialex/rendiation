use crate::*;

pub trait WebGPUBackground: 'static + SceneRenderable {
  fn require_pass_clear(&self) -> Option<webgpu::Color>;
}
define_dyn_trait_downcaster_static!(WebGPUBackground);

impl WebGPUBackground for SolidBackground {
  fn require_pass_clear(&self) -> Option<webgpu::Color> {
    webgpu::Color {
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
  fn require_pass_clear(&self) -> Option<webgpu::Color> {
    None
  }
}

impl SceneRenderable for EnvMapBackground {
  fn render<'a>(
    &self,
    _pass: &mut FrameRenderPass,
    _dispatcher: &dyn RenderComponentAny,
    _camera: &SceneCamera,
    _scene: &SceneRenderResourceGroup,
  ) {
    todo!()
  }
}

struct EnvMapBackgroundGPU {
  texture: GPUCubeTextureView,
  sampler: GPUSamplerView,
}

impl ShadingBackground for EnvMapBackgroundGPU {
  fn shading(
    &self,
    builder: &mut ShaderFragmentBuilderView,
    binding: &mut ShaderBindGroupDirectBuilder,
    direction: Node<Vec3<f32>>,
  ) -> Result<(), ShaderBuildError> {
    let cube = binding.bind_by(&self.texture);
    let sampler = binding.bind_by(&self.sampler);
    cube.sample(sampler, direction);
    builder.register::<DefaultDisplay>(cube.sample(sampler, direction));
    Ok(())
  }
}

struct ShadingBackgroundTask<T> {
  content: T,
}

pub trait ShadingBackground {
  fn shading(
    &self,
    builder: &mut ShaderFragmentBuilderView,
    binding: &mut ShaderBindGroupDirectBuilder,
    direction: Node<Vec3<f32>>,
  ) -> Result<(), ShaderBuildError>;
}

impl<T: ShaderPassBuilder> ShaderPassBuilder for ShadingBackgroundTask<T> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    self.content.setup_pass(ctx)
  }
}

impl<T: ShaderHashProvider> ShaderHashProvider for ShadingBackgroundTask<T> {
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.content.hash_pipeline(hasher)
  }
}

both!(CameraWorldDirection, Vec3<f32>);

impl<T: ShadingBackground> GraphicsShaderProvider for ShadingBackgroundTask<T> {
  fn build(&self, builder: &mut ShaderRenderPipelineBuilder) -> Result<(), ShaderBuildError> {
    builder.vertex(|builder, _| {
      let vertex_index = builder.query::<VertexIndex>()?;
      let projection_inv = builder.query::<CameraProjectionInverseMatrix>()?;
      let view = builder.query::<CameraViewMatrix>()?;

      // hacky way to draw a large triangle
      let tmp1 = vertex_index.into_i32() / val(2);
      let tmp2 = vertex_index.into_i32() & val(1);
      let clip_position = (
        tmp1.into_f32() * val(4.0) - val(1.0),
        tmp2.into_f32() * val(4.0) - val(1.0),
        val(1.0),
        val(1.0),
      )
        .into();

      let model_view: Node<Mat3<f32>> = (view.x().xyz(), view.y().xyz(), view.z().xyz()).into();
      let inv_model_view = model_view.transpose(); // orthonormal

      let unprojected = projection_inv * clip_position;

      let direction = inv_model_view * unprojected.xyz();

      builder.register::<ClipPosition>(clip_position);
      builder.register::<CameraWorldDirection>(direction);
      Ok(())
    })?;

    builder.fragment(|builder, binding| {
      let direction = builder.query::<CameraWorldDirection>()?;
      self.content.shading(builder, binding, direction)?;
      Ok(())
    })
  }
}
