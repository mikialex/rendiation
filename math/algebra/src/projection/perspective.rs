use crate::{Mat4, NDCSpaceMapper, Projection, ResizableProjection, Scalar};

pub struct PerspectiveProjection {
  pub near: f32,
  pub far: f32,
  pub fov: f32,
  pub aspect: f32,
}

impl Default for PerspectiveProjection {
  fn default() -> Self {
    Self {
      near: 1.,
      far: 100_000.,
      fov: 90.,
      aspect: 1.,
    }
  }
}

impl Projection for PerspectiveProjection {
  fn update_projection<S: NDCSpaceMapper>(&self, projection: &mut Mat4<f32>) {
    *projection = Mat4::perspective_fov_aspect::<S>(self.fov, self.aspect, self.near, self.far);
  }
}

impl ResizableProjection for PerspectiveProjection {
  fn resize(&mut self, size: (f32, f32)) {
    self.aspect = size.0 / size.1;
  }
}

impl<T: Scalar> Mat4<T> {
  pub fn perspective_fov_aspect<S: NDCSpaceMapper>(fov: T, aspect: T, near: T, far: T) -> Self {
    let h = T::one() / (fov * T::half() * T::pi_by_c180()).tan();
    let w = h / aspect;
    let q = -far / (far - near);

    #[rustfmt::skip]
    let mat = Mat4::new(
      w,         T::zero(), T::zero(),             T::zero(),
      T::zero(), h,         T::zero(),             T::zero(),
      T::zero(), T::zero(), q,                    -T::one(),
      T::zero(), T::zero(), T::two() * near * q, T::zero(),
    );

    S::from_opengl_standard::<T>() * mat
  }
}
