use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_texture::Size;

use crate::{ResourceWrapped, SceneNode};

pub trait CameraChangeWatcher {
  fn will_change(&mut self, camera: &Camera, id: usize);
  fn will_drop(&mut self, camera: &Camera, id: usize);
}

pub struct SceneCamera {
  pub inner: ResourceWrapped<Camera>,
}

impl std::ops::Deref for SceneCamera {
  type Target = ResourceWrapped<Camera>;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl std::ops::DerefMut for SceneCamera {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

impl SceneCamera {
  pub fn new(p: impl ResizableProjection + RayCaster3<f32> + 'static, node: SceneNode) -> Self {
    let mut inner = Camera {
      bounds: Default::default(),
      projection: Box::new(p),
      projection_matrix: Mat4::one(),
      node,
    };
    inner.projection_changed();

    Self {
      inner: ResourceWrapped::new(inner),
    }
  }

  pub fn resize(&mut self, size: (f32, f32)) {
    self.projection.resize(size);
    self.projection_changed();
  }

  pub fn cast_world_ray(&self, normalized_position: Vec2<f32>) -> Ray3<f32> {
    self.projection.cast_ray(normalized_position)
  }
}

pub struct CameraGroup {
  pub cameras: Vec<SceneCamera>,
  current_rendering_camera: usize,
  /// if no camera provides, we will use default-camera for handling this case easily.
  pub default_camera: SceneCamera,
}

pub struct CameraViewBounds {
  pub width: f32,
  pub height: f32,
  pub to_left: f32,
  pub to_top: f32,
}

impl Default for CameraViewBounds {
  fn default() -> Self {
    Self {
      width: 1.,
      height: 1.,
      to_left: 0.,
      to_top: 0.,
    }
  }
}

pub trait CameraProjection {
  fn update_projection(&self, projection: &mut Mat4<f32>);
  fn resize(&mut self, size: (f32, f32));
  fn pixels_per_unit(&self, distance: f32, view_height: f32) -> f32;
  fn cast_ray(&self, normalized_position: Vec2<f32>) -> Ray3<f32>;
}

impl<T: ResizableProjection + RayCaster3<f32>> CameraProjection for T {
  fn update_projection(&self, projection: &mut Mat4<f32>) {
    self.update_projection::<WebGPU>(projection);
  }
  fn resize(&mut self, size: (f32, f32)) {
    self.resize(size);
  }
  fn pixels_per_unit(&self, distance: f32, view_height: f32) -> f32 {
    self.pixels_per_unit(distance, view_height)
  }

  fn cast_ray(&self, normalized_position: Vec2<f32>) -> Ray3<f32> {
    self.cast_ray(normalized_position)
  }
}

pub struct Camera {
  pub bounds: CameraViewBounds,
  pub projection: Box<dyn CameraProjection>,
  pub projection_matrix: Mat4<f32>,
  pub node: SceneNode,
}

impl Camera {
  pub fn view_size_in_pixel(&self, frame_size: Size) -> Vec2<f32> {
    let width: usize = frame_size.width.into();
    let width = width as f32 * self.bounds.width;
    let height: usize = frame_size.height.into();
    let height = height as f32 * self.bounds.height;
    (width, height).into()
  }

  pub fn projection_changed(&mut self) {
    self
      .projection
      .update_projection(&mut self.projection_matrix);
  }
}
