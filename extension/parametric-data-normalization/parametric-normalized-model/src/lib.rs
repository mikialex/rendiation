//! Normalized parametric model: the source independent representation every CAD input is
//! lowered into. Currently trimmed rational Bézier patches with per occurrence instances, plus
//! source information for debugging. All data is `f64`, conversion to `f32` is the consumer's
//! job. Serialized as `.pnm` files, see `io`.

mod bezier_curve;
mod bezier_surface;
mod io;
mod model;

pub use bezier_curve::*;
pub use bezier_surface::*;
pub use io::*;
pub use model::*;
