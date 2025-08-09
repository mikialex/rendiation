#![feature(impl_trait_in_assoc_type)]

use std::sync::Arc;
use std::task::Context;

use fast_hash_collection::*;
use reactive::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod hook;
pub use hook::*;
mod hook2;
pub use hook2::*;
mod storage;
pub use storage::*;
mod multi_access;
pub use multi_access::*;
mod uniform_group;
pub use uniform_group::*;
mod uniform_array;
pub use uniform_array::*;
mod binding_array;
pub use binding_array::*;
mod cube_map;
pub use cube_map::*;
mod range;
pub use range::*;
mod updater;
pub use updater::*;
mod query_ctx;
use parking_lot::RwLock;
pub use query_ctx::*;
mod task_pool;
pub use task_pool::*;
