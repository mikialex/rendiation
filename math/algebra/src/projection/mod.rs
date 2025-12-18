use crate::*;

mod perspective;
use dyn_clone::DynClone;
pub use perspective::*;
mod orth;
pub use orth::*;

pub trait Projection<T: Scalar>: Send + Sync + DynClone {
  fn compute_projection_mat(&self, mapper: &dyn NDCSpaceMapper<T>) -> Mat4<T>;

  /// Calculate how many screen pixel match one world unit at given distance.
  fn pixels_per_unit(&self, distance: T, view_height_in_pixel: T) -> T;

  fn project(&self, point: Vec3<T>, mapper: &dyn NDCSpaceMapper<T>) -> Vec3<T> {
    (self.compute_projection_mat(mapper) * point.expand_with_one()).xyz()
  }
  fn un_project(&self, point: Vec3<T>, mapper: &dyn NDCSpaceMapper<T>) -> Vec3<T> {
    (self.compute_projection_mat(mapper).inverse_or_identity() * point.expand_with_one()).xyz()
  }
}
impl<T: Scalar> Clone for Box<dyn Projection<T>> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}

pub trait ResizableProjection<T: Scalar>: Projection<T> {
  fn resize(&mut self, size: (T, T));
}

impl<T: Scalar> Mat4<T> {
  /// check if the mat is the perspective, assume the mat is the common projection(perspective or orthographic) in opengl ndc
  pub fn check_is_perspective_matrix_assume_common_projection(&self) -> bool {
    self.c4 == -T::one()
  }

  /// get the near and far assume the mat is the common projection(perspective or orthographic) in opengl ndc
  pub fn get_near_far_assume_is_common_projection(&self) -> (T, T) {
    if self.check_is_perspective_matrix_assume_common_projection() {
      self.get_near_far_assume_perspective()
    } else {
      self.get_near_far_assume_orthographic()
    }
  }

  /// Calculate how many screen pixel match one world unit at given distance.
  ///
  /// If the matrix originated from the common perspective, a better implementation can be used.
  pub fn pixels_per_unit(&self, inverse_of_self: Self, distance: T, view_height_in_pixel: T) -> T {
    let z_in_ndc = *self * Vec3::new(T::zero(), T::zero(), distance);
    let z_in_ndc = z_in_ndc.z;
    let ndc_top = inverse_of_self * Vec3::new(T::zero(), T::one(), z_in_ndc);
    let ndc_bottom = inverse_of_self * Vec3::new(T::zero(), -T::one(), z_in_ndc);

    let real_height = ndc_top.distance_to(ndc_bottom);
    view_height_in_pixel / real_height
  }
}

#[test]
fn test_pixel_per_unit() {
  let distance = 20.;
  let view_height_in_pixel = 100.;

  let p = PerspectiveProjection::<f32>::default();
  let mat = p.compute_projection_mat(&OpenGLxNDC);
  let inv_mat = mat.inverse_or_identity();

  let mat_result = mat.pixels_per_unit(inv_mat, distance, view_height_in_pixel);
  let result = p.pixels_per_unit(distance, view_height_in_pixel);

  assert!(mat_result - result < 0.001);

  let p = OrthographicProjection::<f32>::default();
  let mat = p.compute_projection_mat(&OpenGLxNDC);
  let inv_mat = mat.inverse_or_identity();

  let mat_result = mat.pixels_per_unit(inv_mat, distance, view_height_in_pixel);
  let result = p.pixels_per_unit(distance, view_height_in_pixel);

  assert!(mat_result - result < 0.001);
}
