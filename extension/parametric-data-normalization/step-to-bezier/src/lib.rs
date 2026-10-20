//! STEP B-rep to trimmed rational Bézier patches (`ParametricNormalizedModel`). See `DESIGN.md`
//! for the module contract.

pub mod approx;
pub mod brep;
pub mod curve;
mod error;
pub mod face;
pub mod math;
mod pipeline;
mod report;
pub mod surface;
pub mod trim;
pub mod validate;

pub use error::*;
pub use parametric_normalized_model;
pub use parametric_normalized_model::{
  FaceInstance, ParametricNormalizedModel, TrimCurve, TrimCurveSource, TrimLoop, TrimmedBezierPatch,
};
pub use pipeline::*;
pub use report::*;
pub use step_reader;
