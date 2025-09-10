//! this crate's feature allows user create rw storage buffer from a single buffer pool
//! to workaround the binding limitation on some platform.

use std::num::NonZeroU64;
use std::{marker::PhantomData, sync::Arc};

use parking_lot::RwLock;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

mod combine;
pub(crate) use combine::*;
mod storage;
pub use storage::*;
mod uniform;
pub use uniform::*;
mod maybe_combined;
pub use maybe_combined::*;
mod storage_atomic_array;
pub use storage_atomic_array::*;
