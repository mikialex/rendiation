use rendiation_algebra::*;
use rendiation_texture::Size;
use rendiation_webgpu::*;

use crate::*;

impl CameraViewBounds {
  pub fn setup_viewport<'a>(&self, pass: &mut GPURenderPass<'a>, buffer_size: Size) {
    let width: usize = buffer_size.width.into();
    let width = width as f32;
    let height: usize = buffer_size.height.into();
    let height = height as f32;
    pass.set_viewport(
      width * self.to_left,
      height * self.to_top,
      width * self.width,
      height * self.height,
      0.,
      1.,
    )
  }
}

#[derive(Default)]
pub struct CameraGPUStore {
  inner: ResourceMapper<CameraGPU, Camera>,
}

impl std::ops::Deref for CameraGPUStore {
  type Target = ResourceMapper<CameraGPU, Camera>;

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
    self.get_update_or_insert_with(
      camera,
      |_| CameraGPU::new(gpu),
      |camera_gpu, camera| {
        camera_gpu.update(gpu, camera);
      },
    )
  }

  pub fn expect_gpu(&self, camera: &SceneCamera) -> &CameraGPU {
    self.get_unwrap(camera)
  }
}

pub struct CameraGPU {
  pub ubo: UniformBufferDataView<CameraGPUTransform>,
}

impl ShaderHashProvider for CameraGPU {}

impl ShaderBindingProvider for CameraGPU {
  fn setup_binding(&self, builder: &mut BindingBuilder) {
    builder.setup_uniform(&self.ubo, SB::Camera)
  }
}

impl ShaderGraphProvider for CameraGPU {
  fn build(
    &self,
    builder: &mut ShaderGraphRenderPipelineBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    builder.vertex(|builder, binding| {
      let camera = binding.uniform_by(&self.ubo, SB::Camera).expand();
      let position = builder.query::<WorldVertexPosition>()?.get_last();
      builder.register::<ClipPosition>(camera.projection * camera.view * (position, 1.).into());
      Ok(())
    })
  }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default, ShaderUniform)]
pub struct CameraGPUTransform {
  projection: Mat4<f32>,
  rotation: Mat4<f32>,
  view: Mat4<f32>,
}

impl CameraGPU {
  pub fn update(&mut self, gpu: &GPU, camera: &Camera) -> &mut Self {
    self.ubo.resource.mutate(|uniform| {
      let world_matrix = camera.node.visit(|node| node.local_matrix);
      uniform.view = world_matrix.inverse_or_identity();
      uniform.rotation = world_matrix.extract_rotation_mat();
      uniform.projection = camera.projection_matrix;
    });

    self.ubo.resource.update(&gpu.queue);

    self
  }

  pub fn new(gpu: &GPU) -> Self {
    let ubo =
      UniformBufferDataResource::create_with_source(CameraGPUTransform::default(), &gpu.device);
    let ubo = ubo.create_view(());

    Self { ubo }
  }
}
