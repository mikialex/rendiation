#![feature(associated_type_bounds)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]

pub mod group;
pub use group::*;
pub mod mesh;
pub use mesh::*;
pub mod utils;
pub use utils::*;

pub mod tessellation;
pub mod vertex;

#[cfg(feature = "webgpu")]
pub mod webgpu;
#[cfg(feature = "webgpu")]
pub use webgpu::*;
