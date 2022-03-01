#![feature(capture_disjoint_fields)]
#![feature(array_methods)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(hash_raw_entry)]
#![feature(trait_upcasting)]
#![feature(explicit_generic_args_with_impl_trait)]
#![allow(incomplete_features)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::unit_arg)]

pub mod core;
pub mod util;

pub mod webgpu;
pub use webgpu::*;

pub use crate::core::*;
pub use util::*;

pub use arena::*;
pub use arena_tree::*;

use bytemuck::*;
use shadergraph::*;
