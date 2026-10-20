//! View dependent adaptive tessellation on the GPU (plan stage 1, see `plan.md`). The CPU
//! reference with the same algorithm is the `parametric-tessellation` crate.

mod output;

pub use output::*;
