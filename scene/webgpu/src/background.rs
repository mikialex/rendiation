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
      let cube = binding.uniform_by(&self.texture);
      let sampler = binding.uniform_by(&self.sampler);
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

impl<T: ShadingBackground> ShaderGraphProvider for ShadingBackgroundTask<T> {
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

wgsl_fn!(
  fn background_direction(vertex_index: u32, view: mat4x4<f32>, projection_inv: mat4x4<f32>) -> vec3<f32> {
    // hacky way to draw a large triangle
    let tmp1 = i32(vertex_index) / 2;
    let tmp2 = i32(vertex_index) & 1;
    let pos = vec4<f32>(
      f32(tmp1) * 4.0 - 1.0,
      f32(tmp2) * 4.0 - 1.0,
      1.0,
      1.0
    );

    // transposition = inversion for this orthonormal matrix
    let inv_model_view = transpose(mat3x3<f32>(view.x.xyz, view.y.xyz, view.z.xyz));
    let unprojected = projection_inv * pos;

    return inv_model_view * unprojected.xyz;
  }
);
