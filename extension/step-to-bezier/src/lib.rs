//! STEP B-rep to trimmed rational Bézier patches. See `DESIGN.md` for the module contract.

pub mod approx;
pub mod brep;
pub mod curve;
mod error;
pub mod face;
pub mod math;
mod output;
mod pipeline;
pub mod surface;
pub mod trim;
pub mod validate;

pub use error::*;
pub use output::*;
pub use pipeline::*;
pub use rendiation_step_reader;
