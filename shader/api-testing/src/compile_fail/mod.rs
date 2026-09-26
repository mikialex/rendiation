//! Each case is a `compile_fail` doctest attached to a unit struct, the struct name is the case
//! name. Keep each case minimal, so the expected error code can only come from the tested usage.

pub mod builtin;
pub mod operator;
pub mod subgroup;
pub mod texture;
