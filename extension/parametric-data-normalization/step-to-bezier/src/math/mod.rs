//! Exact f64 geometry kernel: NURBS decomposition, conic arcs, piecewise parameter maps, root
//! finding, projection and fitting. The rational Bézier curve and surface types themselves live
//! in `parametric-normalized-model` and are re-exported here.

mod conic;
mod fit;
mod nurbs;
mod param_map;
mod project;
mod roots;

pub use conic::*;
pub use fit::*;
pub use nurbs::*;
pub use param_map::*;
pub use parametric_normalized_model::{
  Axis, Blend, HomogeneousPoint, RationalBezierCurve, RationalBezierCurve2, RationalBezierCurve3,
  RationalBezierSurface, blend, de_casteljau, de_casteljau_with_derivative, hodograph,
  rational_first_derivative,
};
pub use project::*;
pub use roots::*;
