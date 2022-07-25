use rendiation_algebra::*;

/// old perspective defaults
pub fn make_perspective<T: Scalar>() -> PerspectiveProjection<T> {
  PerspectiveProjection {
    near: T::eval::<{ scalar_transmute(1.0) }>(),
    far: T::eval::<{ scalar_transmute(100_1000.0) }>(),
    fov: Deg::by(T::eval::<{ scalar_transmute(90.0) }>()),
    aspect: T::eval::<{ scalar_transmute(1.0) }>(),
  }
}

// This allows treating the utils as a standalone example,
// thus avoiding listing the example names in `Cargo.toml`.
#[allow(dead_code)]
fn main() {}
