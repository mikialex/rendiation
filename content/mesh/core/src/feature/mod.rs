mod adjacency;
mod bvh;
mod collect;
mod conversion;
mod intersection;

pub use adjacency::*;
pub use bvh::*;
pub use collect::*;
pub use conversion::*;
pub use intersection::*;

/// useful utils in mesh processing, for checking a triangle like array if is degenerated
pub fn triangle_is_not_degenerated<T: PartialEq>(tri: &[T; 3]) -> bool {
  tri[0] != tri[1] && tri[0] != tri[2] && tri[1] != tri[2]
}
