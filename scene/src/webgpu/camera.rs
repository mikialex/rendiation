use rendiation_algebra::*;
use rendiation_webgpu::*;

use crate::*;

impl CameraViewBounds {
  pub fn setup_viewport<'a>(&self, pass: &mut GPURenderPass<'a>) {
    let size = pass.info().buffer_size;
    let width: usize = size.width.into();
    let width = width as f32;
    let height: usize = size.height.into();
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
  pub ubo: UniformBufferData<CameraGPUTransform>,
}

impl ShaderGraphProvider for CameraGPU {
  fn build_vertex(
    &self,
    builder: &mut ShaderGraphVertexBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let camera = builder.uniform_by(&self.ubo, SB::Camera).expand();
    let position = builder.query::<WorldVertexPosition>()?.get_last();
    builder.register::<ClipPosition>(camera.projection * camera.view * (position, 1.).into());
    Ok(())
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
    let uniform: &mut CameraGPUTransform = &mut self.ubo;
    let world_matrix = camera.node.visit(|node| node.local_matrix);
    uniform.view = world_matrix.inverse_or_identity();
    uniform.rotation = world_matrix.extract_rotation_mat();
    uniform.projection = camera.projection_matrix;

    self.ubo.update(&gpu.queue);

    self
  }

  pub fn new(gpu: &GPU) -> Self {
    let device = &gpu.device;

    let ubo: UniformBufferData<CameraGPUTransform> = UniformBufferData::create_default(device);

    Self { ubo }
  }
}
