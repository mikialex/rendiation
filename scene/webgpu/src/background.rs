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
    builder: &mut ShaderGraphRenderPipelineBuilder,
    direction: Node<Vec3<f32>>,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.fragment(|builder, binding| {
      let cube = binding.bind_by(&self.texture);
      let sampler = binding.bind_by(&self.sampler);
      cube.sample(sampler, direction);
      builder.register::<DefaultDisplay>(cube.sample(sampler, direction));
      Ok(())
    })
  }
}

struct ShadingBackgroundTask<T> {
  content: T,
}

pub trait ShadingBackground {
  fn shading(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
    direction: Node<Vec3<f32>>,
  ) -> Result<(), ShaderGraphBuildError>;
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

impl<T: ShadingBackground> GraphicsShaderProvider for ShadingBackgroundTask<T> {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let direction = builder.vertex(|builder, _| {
      let vertex_index = builder.query::<VertexIndex>()?;
      let proj_inv = builder.query::<CameraProjectionInverseMatrix>()?;
      let view = builder.query::<CameraViewMatrix>()?;
      Ok(background_direction(vertex_index, view, proj_inv))
    })?;

    self.content.shading(builder, direction)
  }
}

#[shadergraph_fn]
fn background_direction(
  vertex_index: Node<u32>,
  view: Node<Mat4<f32>>,
  projection_inv: Node<Mat4<f32>>,
) -> Node<Vec3<f32>> {
  // hacky way to draw a large triangle
  let tmp1 = vertex_index.into_i32() / val(2);
  let tmp2 = vertex_index.into_i32() & val(1);
  let pos = (
    tmp1.into_f32() * val(4.0) - val(1.0),
    tmp2.into_f32() * val(4.0) - val(1.0),
    val(1.0),
    val(1.0),
  )
    .into();

  let model_view: Node<Mat3<f32>> = (view.x().xyz(), view.y().xyz(), view.z().xyz()).into();
  let inv_model_view = model_view.transpose(); // orthonormal

  let unprojected = projection_inv * pos;

  inv_model_view * unprojected.xyz()
}
