use crate::{Mat4, NDCSpaceMapper, Projection, ResizableProjection, Scalar};

pub struct OrthographicProjection {
  pub left: f32,
  pub right: f32,
  pub top: f32,
  pub bottom: f32,
  pub near: f32,
  pub far: f32,
}

impl Default for OrthographicProjection {
  fn default() -> Self {
    Self {
      left: -50.0,
      right: 50.0,
      top: 50.0,
      bottom: -50.0,
      near: 0.0,
      far: 1000.0,
    }
  }
}

impl Projection for OrthographicProjection {
  fn update_projection<S: NDCSpaceMapper>(&self, projection: &mut Mat4<f32>) {
    *projection = Mat4::ortho::<S>(
      self.left,
      self.right,
      self.bottom,
      self.top,
      self.near,
      self.far,
    );
  }

  fn pixels_per_unit(&self, _distance: f32, view_height: f32) -> f32 {
    view_height / (self.top - self.bottom).abs()
  }
}

pub struct ViewFrustumOrthographicProjection {
  orth: OrthographicProjection,
  aspect: f32,
  frustum_size: f32,
}

impl ViewFrustumOrthographicProjection {
  pub fn set_aspect(&mut self, aspect: f32) {
    self.aspect = aspect;
    self.update_orth();
  }

  pub fn set_frustum_size(&mut self, frustum_size: f32) {
    self.frustum_size = frustum_size;
    self.update_orth();
  }

  fn update_orth(&mut self) {
    self.orth.left = self.frustum_size * self.aspect / -2.;
    self.orth.right = self.frustum_size * self.aspect / 2.;
    self.orth.top = self.frustum_size / 2.;
    self.orth.bottom = self.frustum_size / -2.;
  }
}

impl Default for ViewFrustumOrthographicProjection {
  fn default() -> Self {
    ViewFrustumOrthographicProjection {
      orth: OrthographicProjection::default(),
      aspect: 1.,
      frustum_size: 50.,
    }
  }
}

impl Projection for ViewFrustumOrthographicProjection {
  fn update_projection<S: NDCSpaceMapper>(&self, projection: &mut Mat4<f32>) {
    self.orth.update_projection::<S>(projection);
  }

  fn pixels_per_unit(&self, distance: f32, view_height: f32) -> f32 {
    self.orth.pixels_per_unit(distance, view_height)
  }
}

impl ResizableProjection for ViewFrustumOrthographicProjection {
  fn resize(&mut self, size: (f32, f32)) {
    self.set_aspect(size.0 / size.1);
  }
}

impl<T: Scalar> Mat4<T> {
  pub fn ortho<S: NDCSpaceMapper>(left: T, right: T, bottom: T, top: T, near: T, far: T) -> Self {
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

    S::from_opengl_standard::<T>() * mat
  }
}
