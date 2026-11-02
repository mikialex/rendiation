use crate::*;

/// The symmetric perspective projection. `near` and `far` are the distance toward the view
/// direction (-z), both of them should be positive.
#[derive(Serialize, Deserialize)]
#[derive(Debug, Copy, Clone, PartialEq, Facet)]
pub struct PerspectiveProjection<T> {
  pub near: T,
  pub far: T,
  /// vertical fov
  pub fov: Deg<T>,
  /// width / height
  pub aspect: T,
}

impl<T: Scalar> Default for PerspectiveProjection<T> {
  fn default() -> Self {
    Self {
      near: T::eval::<{ scalar_transmute(0.1) }>(),
      far: T::eval::<{ scalar_transmute(2000.0) }>(),
      fov: Deg::by(T::eval::<{ scalar_transmute(50.0) }>()),
      aspect: T::eval::<{ scalar_transmute(1.0) }>(),
    }
  }
}

impl<T: Scalar> PerspectiveProjection<T> {
  /// The projection matrix that maps the view space into the target NDC.
  pub fn compute_projection_mat(&self, mapper: &dyn NDCSpaceMapper<T>) -> Mat4<T> {
    Mat4::perspective_fov_aspect(self.fov.to_rad(), self.aspect, self.near, self.far, mapper)
  }

  /// How many screen pixel match one world unit at given distance.
  pub fn pixels_per_unit(&self, distance: T, view_height_in_pixel: T) -> T {
    let view_height_at_dist_one = T::two() * (self.fov.to_rad() / T::two()).tan();
    let distance_when_each_world_unit_match_screen_unit =
      view_height_in_pixel / view_height_at_dist_one;
    distance_when_each_world_unit_match_screen_unit / distance
  }

  /// Update the aspect by the (width, height) of the view.
  pub fn resize(&mut self, size: (T, T)) {
    self.aspect = size.0 / size.1;
  }
}

impl<T: Scalar> Mat4<T> {
  /// Create the perspective projection matrix by the vertical `fov` in radians, same as
  /// `gluPerspective`, then mapped into the target NDC by the mapper.
  ///
  /// <https://www.scratchapixel.com/lessons/3d-basic-rendering/perspective-and-orthographic-projection-matrix/opengl-perspective-projection-matrix>
  pub fn perspective_fov_aspect(
    fov: T,
    aspect: T,
    near: T,
    far: T,
    mapper: &dyn NDCSpaceMapper<T>,
  ) -> Self {
    let h = T::one() / (fov * T::half()).tan();
    let w = h / aspect;
    let c = -(far + near) / (far - near);
    let q = -T::two() * near * far / (far - near);

    #[rustfmt::skip]
    let mat = Mat4::new(
      w,         T::zero(), T::zero(), T::zero(),
      T::zero(), h,         T::zero(), T::zero(),
      T::zero(), T::zero(), c,        -T::one(),
      T::zero(), T::zero(), q,         T::zero(),
    );

    mapper.transform_from_opengl_standard_ndc() * mat
  }

  /// Get the near and far, assume the mat is created by [`Mat4::perspective_fov_aspect`] with
  /// [`OpenGLxNDC`].
  pub fn get_near_far_assume_perspective(&self) -> (T, T) {
    let q = self.d3;
    let c = self.c3;
    let f = q / (c + T::one());
    let n = q / (c - T::one());
    (n, f)
  }
}

#[test]
fn test_near_far() {
  let p = PerspectiveProjection::<f32>::default();
  let mat = p.compute_projection_mat(&OpenGLxNDC);
  let (n, f) = mat.get_near_far_assume_perspective();
  assert!((n - p.near).abs() < 0.001);
  // recovering far from f32 matrix suffers from the precision loss of (c + 1) when near << far
  assert!((f - p.far).abs() / p.far < 0.01, "{f}");
}
