use crate::*;

pub fn setup_viewport<'a>(cb: &CameraViewBounds, pass: &mut GPURenderPass<'a>, buffer_size: Size) {
  let width: usize = buffer_size.width.into();
  let width = width as f32;
  let height: usize = buffer_size.height.into();
  let height = height as f32;
  pass.set_viewport(
    width * cb.to_left,
    height * cb.to_top,
    width * cb.width,
    height * cb.height,
    0.,
    1.,
  )
}

#[derive(Default)]
pub struct CameraGPUStore {
  inner: IdentityMapper<CameraGPU, SceneCameraInner>,
}

impl std::ops::Deref for CameraGPUStore {
  type Target = IdentityMapper<CameraGPU, SceneCameraInner>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl std::ops::DerefMut for CameraGPUStore {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

impl CameraGPUStore {
  pub fn check_update_gpu(&mut self, camera: &SceneCamera, gpu: &GPU) -> &CameraGPU {
    let camera = camera.read();
    self.get_update_or_insert_with(
      &camera,
      |_| CameraGPU::new(gpu),
      |camera_gpu, camera| {
        camera_gpu.update(gpu, camera);
      },
    )
  }

  pub fn expect_gpu(&self, camera: &SceneCamera) -> &CameraGPU {
    let camera = camera.read();
    self.get_unwrap(&camera)
  }
}

pub struct CameraGPU {
  pub ubo: UniformBufferDataView<CameraGPUTransform>,
}

impl ShaderHashProvider for CameraGPU {}

impl ShaderPassBuilder for CameraGPU {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.ubo, SB::Camera)
  }
}

impl ShaderGraphProvider for CameraGPU {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let camera = self.inject_uniforms(builder);

    builder.vertex(|builder, _| {
      let camera = camera.using().expand();
      let position = builder.query::<WorldVertexPosition>()?;

      let jitter = builder.query::<TexelSize>()? * camera.jitter_normalized;
      let jitter = (jitter, 0., 0.).into();

      builder.register::<ClipPosition>(camera.view_projection * (position, 1.).into() + jitter);

      Ok(())
    })
  }
}

#[repr(C)]
#[std140_layout]
#[derive(Clone, Copy, Default, ShaderStruct)]
pub struct CameraGPUTransform {
  pub projection: Mat4<f32>,
  pub projection_inv: Mat4<f32>,

  pub rotation: Mat4<f32>,

  pub view: Mat4<f32>,
  pub world: Mat4<f32>,

  pub view_projection: Mat4<f32>,
  pub view_projection_inv: Mat4<f32>,

  pub jitter_normalized: Vec2<f32>,
}

impl CameraGPUTransform {
  pub fn from_proj_and_world(proj: Mat4<f32>, world: Mat4<f32>) -> Self {
    let mut r = Self::default();
    r.world = world;
    r.view = world.inverse_or_identity();
    r.rotation = world.extract_rotation_mat();
    r.projection = proj;
    r.projection_inv = proj.inverse_or_identity();
    r.view_projection = proj * r.view;
    r.view_projection_inv = r.view_projection.inverse_or_identity();
    r.jitter_normalized = Vec2::zero();
    r
  }
}

impl CameraGPU {
  pub fn inject_uniforms(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> UniformNodePreparer<CameraGPUTransform> {
    builder
      .uniform_by(&self.ubo, SB::Camera)
      .using_both(builder, |r, camera| {
        let camera = camera.expand();
        r.reg::<CameraViewMatrix>(camera.view);
        r.reg::<CameraProjectionMatrix>(camera.projection);
        r.reg::<CameraProjectionInverseMatrix>(camera.projection_inv);
        r.reg::<CameraWorldMatrix>(camera.world);
        r.reg::<CameraViewProjectionMatrix>(camera.view_projection);
        r.reg::<CameraViewProjectionInverseMatrix>(camera.view_projection_inv);
      })
  }

  pub fn update(&mut self, gpu: &GPU, camera: &SceneCameraInner) -> &mut Self {
    self.ubo.resource.mutate(|uniform| {
      let world_matrix = camera.node.visit(|node| node.world_matrix);
      *uniform = CameraGPUTransform::from_proj_and_world(camera.projection_matrix, world_matrix);
    });

    self.ubo.resource.upload(&gpu.queue);
    self
  }

  pub fn new(gpu: &GPU) -> Self {
    Self {
      ubo: create_uniform(CameraGPUTransform::default(), gpu),
    }
  }
}
