//! View dependent adaptive tessellation on the GPU (plan stage 1, see `plan.md`). The CPU
//! reference with the same algorithm is the `parametric-tessellation` crate.

mod data;
mod draw;
mod output;
mod passes;
mod shader_lib;
mod tessellator;
mod trim;

pub use data::*;
pub use draw::*;
pub use output::*;
pub use passes::*;
pub use shader_lib::*;
pub use tessellator::*;
pub use trim::*;

#[cfg(test)]
mod tests_lib;

#[cfg(test)]
mod tests_frame;

#[cfg(test)]
mod tests_draw;

#[cfg(test)]
mod tests_trim;
