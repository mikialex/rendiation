use rendiation_algebra::SquareMatrix;
use rendiation_ral::{ResourceManager, UniformHandle, Viewport};
use rendiation_render_entity::{Camera, PerspectiveProjection, Projection, ResizableProjection};
use rendiation_shader_library::transform::CameraTransform;
use rendiation_webgpu::{WebGPU, OPENGL_TO_WGPU_MATRIX};

pub struct VoxlandCamera {
  viewport: Viewport,
  camera: Camera,
  projection: PerspectiveProjection,
  gpu: UniformHandle<WebGPU, CameraTransform>,
}

impl VoxlandCamera {
  pub fn new(resource: &mut ResourceManager<WebGPU>, view_size: (usize, usize)) -> Self {
    Self {
      viewport: Viewport::new(view_size),
      camera: Camera::new(),
      projection: PerspectiveProjection::default(),
      gpu: resource
        .bindable
        .uniform_buffers
        .add(CameraTransform::default()),
    }
  }

  pub fn resize(&mut self, view_size: (usize, usize)) {
    self
      .viewport
      .set_size(view_size.0 as f32, view_size.1 as f32);

    self
      .projection
      .resize((view_size.0 as f32, view_size.1 as f32));
  }

  pub fn camera(&self) -> &Camera {
    &self.camera
  }

  pub fn camera_mut(&mut self) -> &mut Camera {
    &mut self.camera
  }

  pub fn gpu_handle(&self) -> UniformHandle<WebGPU, CameraTransform> {
    self.gpu
  }

  pub fn update(&mut self, res: &mut ResourceManager<WebGPU>) {
    self
      .projection
      .update_projection(&mut self.camera.projection_matrix);
    self.camera.matrix_inverse = self.camera.matrix.inverse_or_identity();
    res.bindable.uniform_buffers.update(
      self.gpu,
      CameraTransform {
        projection_matrix: OPENGL_TO_WGPU_MATRIX * self.camera.projection_matrix,
      },
    );
  }
}
