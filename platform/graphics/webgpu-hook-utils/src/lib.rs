#![feature(impl_trait_in_assoc_type)]

use std::sync::Arc;

use fast_hash_collection::*;
use parking_lot::RwLock;
pub use query_hook::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod hook;
pub use hook::*;
mod use_result_ext;
pub use use_result_ext::*;
mod storage_util;
pub use storage_util::*;
mod multi_access;
pub use multi_access::*;
mod binding_array;
pub use binding_array::*;
mod sparse_buffer_writes;
pub use sparse_buffer_writes::*;
mod range;
pub use range::*;

pub type UniformArray<T, const N: usize> = UniformBufferDataView<Shader140Array<T, N>>;
