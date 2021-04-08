use crate::{Mat4, Projection, ResizableProjection, Scalar};

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
  fn update_projection(&self, projection: &mut Mat4<f32>) {
    *projection = Mat4::ortho(
      self.left,
      self.right,
      self.bottom,
      self.top,
      self.near,
      self.far,
    );
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
  fn update_projection(&self, projection: &mut Mat4<f32>) {
    self.orth.update_projection(projection);
  }
}

impl ResizableProjection for ViewFrustumOrthographicProjection {
  fn resize(&mut self, size: (f32, f32)) {
    self.set_aspect(size.0 / size.1);
  }
}

impl<T: Scalar> Mat4<T> {
  pub fn ortho_lh(left: T, right: T, bottom: T, top: T, znear: T, zfar: T) -> Self {
    let tx = -(right + left) / (right - left);
    let ty = -(top + bottom) / (top - bottom);
    let tz = -znear / (zfar - znear);
    let cx = T::two() / (right - left);
    let cy = T::two() / (top - bottom);
    let cz = T::two() / (zfar - znear);

    #[rustfmt::skip]
    Mat4::new(
      cx,        T::zero(), T::zero(), T::zero(),
      T::zero(), cy,        T::zero(), T::zero(),
      T::zero(), T::zero(), cz,        T::zero(),
      tx,        ty,        tz,        T::one(),
    )
  }

  pub fn ortho_rh(left: T, right: T, bottom: T, top: T, znear: T, zfar: T) -> Self {
    let tx = -(right + left) / (right - left);
    let ty = -(top + bottom) / (top - bottom);
    let tz = -(zfar + znear) / (zfar - znear);
    let cx = T::two() / (right - left);
    let cy = T::two() / (top - bottom);
    let cz = -T::two() / (zfar - znear);

    #[rustfmt::skip]
    Mat4::new(
      cx,        T::zero(), T::zero(), T::zero(),
      T::zero(), cy,        T::zero(), T::zero(),
      T::zero(), T::zero(), cz,        T::zero(),
      tx,        ty,        tz,        T::one(),
    )
  }

  pub fn ortho(left: T, right: T, bottom: T, top: T, znear: T, zfar: T) -> Self {
    let w = T::one() / (right - left);
    let h = T::one() / (top - bottom);
    let p = T::one() / (zfar - znear);

    let x = (right + left) * w;
    let y = (top + bottom) * h;
    let z = (zfar + znear) * p;

    #[rustfmt::skip]
    Mat4::new(
      T::two() * w, T::zero(),    T::zero(),    T::zero(),
      T::zero(),    T::two() * h, T::zero(),    T::zero(),
      T::zero(),    T::zero(),   -T::two() * p, T::zero(),
      -x,           -y,           -z,           T::one(),
    )
  }
}
