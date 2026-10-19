//! Exact f64 geometry kernel: rational Bézier curves and surfaces, NURBS decomposition, conic
//! arcs, piecewise parameter maps, root finding, projection and fitting.

mod bezier_curve;
mod bezier_surface;
mod conic;
mod fit;
mod nurbs;
mod param_map;
mod project;
mod roots;

pub use bezier_curve::*;
pub use bezier_surface::*;
pub use conic::*;
pub use fit::*;
pub use nurbs::*;
pub use param_map::*;
pub use project::*;
pub use roots::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
  U,
  V,
}

impl Axis {
  pub fn index(self) -> usize {
    match self {
      Axis::U => 0,
      Axis::V => 1,
    }
  }

  pub fn other(self) -> Self {
    match self {
      Axis::U => Axis::V,
      Axis::V => Axis::U,
    }
  }
}
