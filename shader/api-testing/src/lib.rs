//! Tests of the shader EDSL(rendiation-shader-api), no GPU is required.
//!
//! The tests are grouped by topic, each topic is tested in two ways:
//! - compile fail: the WGSL invalid code must be rejected by the rust type system. Each case is a
//!   `compile_fail` doctest in the `compile_fail` module, which is compiled independently and
//!   checks the expected error code.
//! - validation: the WGSL valid code must compile and the built shader must pass the naga
//!   validation. The build time checks of the EDSL are tested here too.
//!
//! Run all of them by `cargo test -p rendiation-shader-api-testing`, the doctests are not included
//! when `--lib` is specified.

#[cfg(doctest)]
pub mod compile_fail;

#[cfg(test)]
mod harness;

#[cfg(test)]
mod builtin;
#[cfg(test)]
mod control_flow;
#[cfg(test)]
mod operator;
#[cfg(test)]
mod subgroup;
#[cfg(test)]
mod texture;
