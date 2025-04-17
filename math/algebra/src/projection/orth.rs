use crate::*;

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
      far: T::eval::<{ scalar_transmute(1000.0) }>(),
    }
  }
}

impl<T: Scalar> Projection<T> for OrthographicProjection<T> {
  fn compute_projection_mat(&self, mapper: &dyn NDCSpaceMapper<T>) -> Mat4<T> {
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

  fn pixels_per_unit(&self, _distance: T, view_height_in_pixel: T) -> T {
    view_height_in_pixel / (self.top - self.bottom).abs()
  }
}

#[derive(Debug, Copy, Clone, Facet)]
pub struct ViewFrustumOrthographicProjection<T> {
  orth: OrthographicProjection<T>,
  aspect: T,
  frustum_size: T,
}

impl<T: Scalar> ViewFrustumOrthographicProjection<T> {
  pub fn get_orth(&self) -> &OrthographicProjection<T> {
    &self.orth
  }

  pub fn set_near_far(&mut self, near: T, far: T) {
    self.orth.near = near;
    self.orth.far = far;
  }

  pub fn set_aspect(&mut self, aspect: T) {
    self.aspect = aspect;
    self.update_orth();
  }

  pub fn set_frustum_size(&mut self, frustum_size: T) {
    self.frustum_size = frustum_size;
    self.update_orth();
  }

  fn update_orth(&mut self) {
    self.orth.left = self.frustum_size * self.aspect / -T::two();
    self.orth.right = self.frustum_size * self.aspect / T::two();
    self.orth.top = self.frustum_size / T::two();
    self.orth.bottom = self.frustum_size / -T::two();
  }
}

impl<T: Scalar> Default for ViewFrustumOrthographicProjection<T> {
  fn default() -> Self {
    ViewFrustumOrthographicProjection {
      orth: OrthographicProjection::default(),
      aspect: T::one(),
      frustum_size: T::eval::<{ scalar_transmute(50.0) }>(),
    }
  }
}

impl<T: Scalar> Projection<T> for ViewFrustumOrthographicProjection<T> {
  fn compute_projection_mat(&self, mapper: &dyn NDCSpaceMapper<T>) -> Mat4<T> {
    self.orth.compute_projection_mat(mapper)
  }

  fn pixels_per_unit(&self, distance: T, view_height_in_pixel: T) -> T {
    self.orth.pixels_per_unit(distance, view_height_in_pixel)
  }
}

impl<T: Scalar> ResizableProjection<T> for ViewFrustumOrthographicProjection<T> {
  fn resize(&mut self, size: (T, T)) {
    self.set_aspect(size.0 / size.1);
  }
}

impl<T: Scalar> Mat4<T> {
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
}
