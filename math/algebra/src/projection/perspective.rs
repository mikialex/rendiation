use crate::*;

#[derive(Debug, Copy, Clone)]
pub struct PerspectiveProjection<T> {
  pub near: T,
  pub far: T,
  pub fov: Deg<T>,
  pub aspect: T,
}

impl<T: Scalar> Default for PerspectiveProjection<T> {
  fn default() -> Self {
    Self {
      near: T::eval::<1.>(),
      far: T::eval::<100_000.>(),
      fov: Deg::by(T::eval::<90.>()),
      aspect: T::eval::<1.>(),
    }
  }
}

impl<T: Scalar> Projection<T> for PerspectiveProjection<T> {
  fn update_projection<S: NDCSpaceMapper>(&self, projection: &mut Mat4<T>) {
    *projection =
      Mat4::perspective_fov_aspect::<S>(self.fov.to_rad(), self.aspect, self.near, self.far);
  }

  fn pixels_per_unit(&self, distance: T, view_height: T) -> T {
    let pixels_of_dist_one = T::two() * (self.fov.to_rad() / T::two()).tan();
    let distance_when_each_world_unit_match_screen_unit = view_height / pixels_of_dist_one;
    distance_when_each_world_unit_match_screen_unit / distance
  }
}

impl<T: Scalar> ResizableProjection<T> for PerspectiveProjection<T> {
  fn resize(&mut self, size: (T, T)) {
    self.aspect = size.0 / size.1;
  }
}

impl<T: Scalar> Mat4<T> {
  pub fn perspective_fov_aspect<S: NDCSpaceMapper>(fov: T, aspect: T, near: T, far: T) -> Self {
    let h = T::one() / (fov * T::half()).tan();
    let w = h / aspect;
    let q = -far / (far - near);

    #[rustfmt::skip]
    let mat = Mat4::new(
      w,         T::zero(), T::zero(),           T::zero(),
      T::zero(), h,         T::zero(),           T::zero(),
      T::zero(), T::zero(), q,                  -T::one(),
      T::zero(), T::zero(), T::two() * near * q, T::zero(),
    );

    S::from_opengl_standard::<T>() * mat
  }
}
