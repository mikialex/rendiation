#![feature(associated_type_bounds)]
#![feature(type_alias_impl_trait)]
#![feature(stmt_expr_attributes)]
#![feature(iterator_try_collect)]

mod group;
pub use group::*;
mod mesh;
pub use mesh::*;
mod utils;
pub use utils::*;

pub mod vertex;
