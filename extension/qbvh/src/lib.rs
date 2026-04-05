use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
use std::{
  collections::BinaryHeap,
  sync::{RwLock, TryLockError},
};

/// The number of lanes of a SIMD number.
const SIMD_WIDTH: usize = 4;
type SimdRealValue = simba::simd::AutoF32x4;
type SimdBoolValue = simba::simd::AutoBoolx4;

use rendiation_abstract_tree::AbstractTreeNode;
use rendiation_algebra::*;
use simba::simd::SimdBool;
use simba::simd::SimdPartialOrd;
use simba::simd::SimdValue;
use smallvec::SmallVec;

pub mod node;
pub use node::*;
pub mod qbvh_impl;
pub use qbvh_impl::*;
pub mod traversal;

#[allow(unused_imports)]
pub use traversal::*;
pub mod visitor;
pub use visitor::*;
pub mod update;
pub use update::*;
pub mod splitter;
pub use splitter::*;
pub mod build;
pub use build::*;
pub mod iter;
pub use iter::*;
pub mod simd;
pub mod test;
pub use simd::*;

#[macro_export]
macro_rules! array(
  ($callback: expr; SIMD_WIDTH) => {
    {
      #[inline(always)]
      #[allow(dead_code)]
      fn create_arr<T>(mut callback: impl FnMut(usize) -> T) -> [T; SIMD_WIDTH] {
        [callback(0usize), callback(1usize), callback(2usize), callback(3usize)]
      }

      create_arr($callback)
    }
  }
);
