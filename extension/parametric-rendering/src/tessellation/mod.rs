//! View dependent adaptive tessellation on the GPU (plan stage 1, see `plan.md`). The CPU
//! reference with the same algorithm is the `parametric-tessellation` crate.

mod draw;
mod output;

pub use draw::*;
pub use output::*;
