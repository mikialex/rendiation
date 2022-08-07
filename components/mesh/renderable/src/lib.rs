#![feature(associated_type_bounds)]
#![feature(type_alias_impl_trait)]

pub mod group;
pub mod mesh;
pub mod tessellation;
pub mod vertex;

#[cfg(feature = "webgpu")]
pub mod webgpu;
#[cfg(feature = "webgpu")]
pub use webgpu::*;
