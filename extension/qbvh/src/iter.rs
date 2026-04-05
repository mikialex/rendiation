use std::{collections::BinaryHeap, ops::DerefMut, sync::RwLockWriteGuard};

use heapless::binary_heap::Max;
use simba::simd::SimdBool;
use smallvec::SmallVec;

use super::*;

pub trait SimdVisitDecider<SimdBV> {
  fn visit(&mut self, bv: &SimdBV, margin: &SimdRealValue) -> SimdVisitStatus;
}

impl<SimdBV, F> SimdVisitDecider<SimdBV> for F
where
  F: FnMut(&SimdBV, &SimdRealValue) -> SimdVisitStatus,
{
  fn visit(&mut self, bv: &SimdBV, margin: &SimdRealValue) -> SimdVisitStatus {
    (self)(bv, margin)
  }
}

pub struct TraverseAll;
impl<SimdBV> SimdVisitDecider<SimdBV> for TraverseAll {
  fn visit(&mut self, _bv: &SimdBV, _margin: &SimdRealValue) -> SimdVisitStatus {
    SimdVisitStatus::MaybeContinue([true, true, true, true].into())
  }
}

pub trait SimdBestFirstVisitDecider<SimdBV> {
  fn visit(&mut self, bv: &SimdBV, margin: &SimdRealValue) -> SimdBestFirstVisitStatus<()>;
}

pub struct LeafDataIter<'a, LeafData, F, BV, SimdBV> {
  pub(crate) stack: RwLockWriteGuard<'a, Vec<u32>>,
  pub(crate) leaf_cache: SmallVec<[LeafData; SIMD_WIDTH]>,
  pub(crate) visit_decider: F,
  pub(crate) qbvh: &'a Qbvh<LeafData, BV, SimdBV>,
}

impl<'a, LeafData, F, BV, SimdBV> Drop for LeafDataIter<'a, LeafData, F, BV, SimdBV> {
  fn drop(&mut self) {
    self.stack.clear()
  }
}

impl<'a, LeafData: Copy, F, BV, SimdBV: Copy> Iterator for LeafDataIter<'a, LeafData, F, BV, SimdBV>
where
  F: SimdVisitDecider<SimdBV>,
{
  type Item = LeafData;
  fn next(&mut self) -> Option<Self::Item> {
    // pop the leaf data first if there is any.
    if let Some(leaf) = self.leaf_cache.pop() {
      return Some(leaf);
    }

    // leaf data cache is empty, find new leaf data group.
    while self.leaf_cache.is_empty() {
      if let Some(entry) = self.stack.pop() {
        let node = self.qbvh.nodes[entry as usize];
        match self.visit_decider.visit(&node.simd_aabb, &node.simd_margin) {
          SimdVisitStatus::ExitEarly => break,
          SimdVisitStatus::MaybeContinue(mask) => {
            let bitmask = mask.bitmask();
            if !node.is_leaf() {
              for lane in 0..SIMD_WIDTH {
                if (bitmask & (1 << lane)) != 0
                  && node.children[lane] as usize <= self.qbvh.nodes.len()
                {
                  self.stack.push(node.children[lane]);
                }
              }
            } else {
              for lane in 0..SIMD_WIDTH {
                if (bitmask & (1 << lane)) != 0 {
                  if let Some(leaf_data) = self.qbvh.proxies.get(node.children[lane] as usize) {
                    self.leaf_cache.push(leaf_data.data);
                  }
                }
              }
            }
          }
        }
      } else {
        break;
      }
    }

    self.leaf_cache.pop()
  }
}

pub struct LeafDataWeightedIter<'a, Stack, LeafData, F, B, SimdBV>
where
  Stack: DerefMut<Target = BinaryHeap<WeightedValue<u32>>>,
{
  pub(crate) stack: Stack,
  pub(crate) leaf_stack: heapless::BinaryHeap<WeightedValue<LeafData>, Max, SIMD_WIDTH>,
  pub(crate) visit_decider: F,
  pub(crate) qbvh: &'a Qbvh<LeafData, B, SimdBV>,
}

impl<'a, Stack, LeafData, F, B, SimdBV> Drop
  for LeafDataWeightedIter<'a, Stack, LeafData, F, B, SimdBV>
where
  Stack: DerefMut<Target = BinaryHeap<WeightedValue<u32>>>,
{
  fn drop(&mut self) {
    self.stack.clear()
  }
}

impl<'a, Stack, LeafData: Copy, F, B, SimdBV> Iterator
  for LeafDataWeightedIter<'a, Stack, LeafData, F, B, SimdBV>
where
  Stack: DerefMut<Target = BinaryHeap<WeightedValue<u32>>>,
  F: SimdBestFirstVisitDecider<SimdBV>,
{
  type Item = (f32, LeafData);
  fn next(&mut self) -> Option<Self::Item> {
    if let Some(leaf) = self.leaf_stack.pop() {
      return Some((leaf.cost, leaf.value));
    }

    while self.leaf_stack.is_empty() {
      match self.stack.pop() {
        None => break,
        Some(entry) => {
          let node = &self.qbvh.nodes[entry.value as usize];
          match self.visit_decider.visit(&node.simd_aabb, &node.simd_margin) {
            SimdBestFirstVisitStatus::ExitEarly(_) => break,
            SimdBestFirstVisitStatus::MaybeContinue { weights, mask, .. } => {
              let bitmask = mask.bitmask();
              let weights: [f32; SIMD_WIDTH] = weights.into();

              #[allow(clippy::needless_range_loop)]
              for lane in 0..SIMD_WIDTH {
                if (bitmask & (1 << lane)) != 0 {
                  if node.is_leaf() {
                    if let Some(proxy) = self.qbvh.proxies.get(node.children[lane] as usize) {
                      self
                        .leaf_stack
                        .push(WeightedValue::new(proxy.data, -weights[lane]))
                        .ok();
                    }
                  } else if (node.children[lane] as usize) < self.qbvh.nodes.len() {
                    self
                      .stack
                      .push(WeightedValue::new(node.children[lane], -weights[lane]));
                  }
                }
              }
            }
          }
        }
      }
    }

    self.leaf_stack.pop().map(|leaf| (-leaf.cost, leaf.value))
  }
}
