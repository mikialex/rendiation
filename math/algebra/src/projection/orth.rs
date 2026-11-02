use crate::*;

/// The orthographic projection, the frustum is the box defined by the view space bounds. `near`
/// and `far` are the distance toward the view direction (-z).
#[derive(Serialize, Deserialize)]
#[derive(Debug, Copy, Clone, PartialEq, Facet)]
pub struct OrthographicProjection<T> {
  pub left: T,
  pub right: T,
  pub top: T,
  pub bottom: T,
  pub near: T,
  pub far: T,
}

impl<T: Scalar> Default for OrthographicProjection<T> {
  fn default() -> Self {
    Self {
      left: T::eval::<{ scalar_transmute(-50.0) }>(),
      right: T::eval::<{ scalar_transmute(50.0) }>(),
      top: T::eval::<{ scalar_transmute(50.0) }>(),
      bottom: T::eval::<{ scalar_transmute(-50.0) }>(),
      near: T::zero(),
      far: T::eval::<{ scalar_transmute(2000.0) }>(),
    }
  }
}

impl<T: Scalar> OrthographicProjection<T> {
  /// The projection matrix that maps the view space into the target NDC.
  pub fn compute_projection_mat(&self, mapper: &dyn NDCSpaceMapper<T>) -> Mat4<T> {
    Mat4::ortho(
      self.left,
      self.right,
      self.bottom,
      self.top,
      self.near,
      self.far,
      mapper,
    )
  }

  /// How many screen pixel match one world unit, which is irrelevant to distance.
  pub fn pixels_per_unit(&self, _distance: T, view_height_in_pixel: T) -> T {
    view_height_in_pixel / (self.top - self.bottom).abs()
  }

  /// The width and height of the view box.
  pub fn size(&self) -> Vec2<T> {
    Vec2::new(self.right - self.left, self.top - self.bottom)
  }

  /// The center of the view box in xy plane.
  pub fn center(&self) -> Vec2<T> {
    Vec2::new(self.right + self.left, self.top + self.bottom).map(|v| v / T::two())
  }

  /// Scale the view box size while keeping the center.
  pub fn scale_from_center(&mut self, scale: Vec2<T>) {
    let new_size_half = self.size() * scale / T::two();
    let center = self.center();

    self.left = center.x - new_size_half.x;
    self.right = center.x + new_size_half.x;
    self.top = center.y + new_size_half.y;
    self.bottom = center.y - new_size_half.y;
  }
}

impl<T: Scalar> Mat4<T> {
  /// Create the orthographic projection matrix, same as `glOrtho`, then mapped into the target
  /// NDC by the mapper.
  pub fn ortho(
    left: T,
    right: T,
    bottom: T,
    top: T,
    near: T,
    far: T,
    mapper: &dyn NDCSpaceMapper<T>,
  ) -> Self {
    let w = T::one() / (right - left);
    let h = T::one() / (top - bottom);
    let p = T::one() / (far - near);

    let x = (right + left) * w;
    let y = (top + bottom) * h;
    let z = (far + near) * p;

    #[rustfmt::skip]
    let mat = Mat4::new(
      T::two() * w, T::zero(),    T::zero(),    T::zero(),
      T::zero(),    T::two() * h, T::zero(),    T::zero(),
      T::zero(),    T::zero(),   -T::two() * p, T::zero(),
      -x,           -y,           -z,           T::one(),
    );

    mapper.transform_from_opengl_standard_ndc() * mat
  }

  /// Get the near and far, assume the mat is created by [`Mat4::ortho`] with [`OpenGLxNDC`].
  pub fn get_near_far_assume_orthographic(&self) -> (T, T) {
    let near = (T::one() + self.d3) / self.c3;
    let far = -(T::one() - self.d3) / self.c3;
    (near, far)
  }
}

#[test]
fn test_near_far() {
  let p = OrthographicProjection::<f32>::default();
  let mat = p.compute_projection_mat(&OpenGLxNDC);
  let (n, f) = mat.get_near_far_assume_orthographic();
  assert!((n - p.near).abs() < 0.001);
  assert!((f - p.far).abs() < 0.1);
}
