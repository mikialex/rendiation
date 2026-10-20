//! View dependent adaptive tessellation on the GPU (plan stage 1, see `plan.md`). The CPU
//! reference with the same algorithm is the `parametric-tessellation` crate.

mod data;
mod output;
mod shader_lib;

pub use data::*;
pub use output::*;
pub use shader_lib::*;

#[cfg(test)]
mod tests_lib;
