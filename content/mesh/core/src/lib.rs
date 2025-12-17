#![feature(stmt_expr_attributes)]
#![feature(iterator_try_collect)]
#![feature(iter_array_chunks)]

mod common_vertex;
mod container;
mod feature;
mod primitive;
mod utils;

use std::{
  any::{Any, TypeId},
  hash::Hash,
  marker::PhantomData,
  num::NonZeroU64,
  sync::Arc,
};

pub use common_vertex::*;
pub use container::*;
use facet::*;
use fast_hash_collection::*;
pub use feature::*;
pub use primitive::*;
use rendiation_algebra::*;
use rendiation_geometry::*;
use serde::*;
use smallvec::SmallVec;
pub use utils::*;
