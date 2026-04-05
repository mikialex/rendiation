use crate::*;

impl<LeafData: Copy, B, SimdBV> Qbvh<LeafData, B, SimdBV> {
  /// Performs a depth-first traversal on the BVH.
  ///
  /// # Return
  ///
  /// Returns `false` if the traversal exited early, and `true` otherwise.
  pub fn traverse_depth_first(
    &self,
    visitor: &mut impl SimdVisitor<LeafData, SimdBV, SimdRealValue>,
  ) -> bool {
    self.traverse_depth_first_node(visitor, 0)
  }

  /// Performs a depth-first traversal on the BVH, starting at the given node.
  ///
  /// # Return
  ///
  /// Returns `false` if the traversal exited early, and `true` otherwise.
  pub fn traverse_depth_first_node(
    &self,
    visitor: &mut impl SimdVisitor<LeafData, SimdBV, SimdRealValue>,
    start_node: u32,
  ) -> bool {
    traverse_impl::traverse_depth_first_node_with_stack(self, visitor, &mut Vec::new(), start_node)
  }

  /// Performs a best-first-search on the BVH.
  ///
  /// Returns the content of the leaf with the smallest associated cost, and a result of
  /// user-defined type.
  pub fn traverse_best_first<V>(&self, visitor: &mut V) -> Option<(NodeIndex, V::Result)>
  where
    V: SimdBestFirstVisitor<LeafData, SimdBV, SimdRealValue>,
    V::Result: Clone, // Because we cannot move out of an array…
  {
    traverse_impl::traverse_best_first_node(self, visitor, 0, f32::MAX)
  }

  pub fn leaf_data_iter<F>(&self, decider: F) -> LeafDataIter<'_, LeafData, F, B, SimdBV> {
    let mut iter = LeafDataIter::new(decider, self);
    if !self.nodes.is_empty() {
      iter.join(0_u32)
    }
    iter
  }

  pub fn leaf_data_weighted_iter<'a, F: SimdBestFirstVisitDecider<SimdBV> + 'a>(
    &'a self,
    init_weight: f32,
    decider: F,
  ) -> impl Iterator<Item = (f32, LeafData)> + 'a {
    use once_cell::sync::Lazy;
    static LOCAL_BINARY_HEAP: Lazy<RwLock<BinaryHeap<WeightedValue<u32>>>> =
      Lazy::new(|| RwLock::new(BinaryHeap::with_capacity(1000)));

    let mut guard = LOCAL_BINARY_HEAP.write().unwrap();
    debug_assert_eq!(guard.len(), 0);

    if !self.nodes.is_empty() {
      guard.push(WeightedValue::new(0_u32, -init_weight / 2.0));
    }

    LeafDataWeightedIter {
      stack: guard,
      leaf_stack: heapless::BinaryHeap::new(),
      visit_decider: decider,
      qbvh: self,
    }
  }
}

impl<'a, LeafData, F, BV, SimdBV> LeafDataIter<'a, LeafData, F, BV, SimdBV> {
  pub fn new(visit_decider: F, qbvh: &'a Qbvh<LeafData, BV, SimdBV>) -> Self {
    use once_cell::sync::Lazy;
    static LOCAL_STACK_1: Lazy<RwLock<Vec<u32>>> =
      Lazy::new(|| RwLock::new(Vec::with_capacity(1000)));
    static LOCAL_STACK_2: Lazy<RwLock<Vec<u32>>> =
      Lazy::new(|| RwLock::new(Vec::with_capacity(1000)));

    // try acquiring another stack if the first stack is in use.
    // for now we not allow creating more than two iter in one thread.
    let guard = match LOCAL_STACK_1.try_write() {
      Ok(g) => g,
      Err(TryLockError::Poisoned(g)) => g.into_inner(),
      Err(TryLockError::WouldBlock) => LOCAL_STACK_2.write().unwrap(),
    };
    debug_assert_eq!(guard.len(), 0);

    Self {
      stack: guard,
      leaf_cache: SmallVec::new(),
      visit_decider,
      qbvh,
    }
  }

  pub fn join(&mut self, node_id: u32) {
    self.stack.push(node_id);
  }
}

mod traverse_impl {
  use super::*;
  /// Performs a depth-first traversal on the BVH.
  ///
  /// # Return
  ///
  /// Returns `false` if the traversal exited early, and `true` otherwise.
  pub(super) fn traverse_depth_first_node_with_stack<LeafData, B, SimdBV>(
    qbvh: &Qbvh<LeafData, B, SimdBV>,
    visitor: &mut impl SimdVisitor<LeafData, SimdBV, SimdRealValue>,
    stack: &mut Vec<u32>,
    start_node: u32,
  ) -> bool {
    stack.clear();

    if !qbvh.nodes.is_empty() {
      stack.push(start_node);
    }
    while let Some(entry) = stack.pop() {
      let node = &qbvh.nodes[entry as usize];
      let leaf_data = if node.is_leaf() {
        Some(array![|ii| Some(&qbvh.proxies.get(node.children[ii] as usize)?.data); SIMD_WIDTH])
      } else {
        None
      };

      match visitor.visit(entry, &node.simd_aabb, &node.simd_margin, leaf_data) {
        SimdVisitStatus::ExitEarly => {
          return false;
        }
        SimdVisitStatus::MaybeContinue(mask) => {
          let bitmask = mask.bitmask();

          if !node.is_leaf() {
            for lane in 0..SIMD_WIDTH {
              if (bitmask & (1 << lane)) != 0 && node.children[lane] as usize <= qbvh.nodes.len() {
                stack.push(node.children[lane]);
              }
            }
          }
        }
      }
    }

    true
  }

  /// Performs a best-first-search on the BVH, starting at the given node.
  ///
  /// Returns the content of the leaf with the smallest associated cost, and a result of
  /// user-defined type.
  pub(super) fn traverse_best_first_node<V, LeafData, B, SimdBV>(
    qbvh: &Qbvh<LeafData, B, SimdBV>,
    visitor: &mut V,
    start_node: u32,
    init_cost: f32,
  ) -> Option<(NodeIndex, V::Result)>
  where
    V: SimdBestFirstVisitor<LeafData, SimdBV, SimdRealValue>,
    V::Result: Clone, // Because we cannot move out of an array…
  {
    if qbvh.nodes.is_empty() {
      return None;
    }

    let mut queue: BinaryHeap<WeightedValue<u32>> = BinaryHeap::new();

    let mut best_cost = init_cost;
    let mut best_result = None;
    queue.push(WeightedValue::new(start_node, -best_cost / 2.0));

    while let Some(entry) = queue.pop() {
      if -entry.cost >= best_cost {
        // No BV left in the tree that has a lower cost than best_result
        break; // Solution found.
      }

      let node = &qbvh.nodes[entry.value as usize];
      let leaf_data = if node.is_leaf() {
        Some(array![|ii| Some(&qbvh.proxies.get(node.children[ii] as usize)?.data); SIMD_WIDTH])
      } else {
        None
      };

      match visitor.visit(best_cost, &node.simd_aabb, &node.simd_margin, leaf_data) {
        SimdBestFirstVisitStatus::ExitEarly(result) => {
          return result.map(|r| (node.parent, r)).or(best_result);
        }
        SimdBestFirstVisitStatus::MaybeContinue {
          weights,
          mask,
          results,
        } => {
          let bitmask = mask.bitmask();
          let weights: [f32; SIMD_WIDTH] = weights.into();

          for ii in 0..SIMD_WIDTH {
            if (bitmask & (1 << ii)) != 0 {
              if node.is_leaf() {
                if weights[ii] < best_cost && results[ii].is_some() {
                  // We found a leaf!
                  if let Some(proxy) = qbvh.proxies.get(node.children[ii] as usize) {
                    best_cost = weights[ii];
                    best_result = Some((proxy.node, results[ii].clone().unwrap()))
                  }
                }
              } else {
                // Internal node, visit the child.
                // Un fortunately, we have this check because invalid Aabbs
                // return a hit as well.
                if (node.children[ii] as usize) < qbvh.nodes.len() {
                  queue.push(WeightedValue::new(node.children[ii], -weights[ii]));
                }
              }
            }
          }
        }
      }
    }

    best_result
  }
}
