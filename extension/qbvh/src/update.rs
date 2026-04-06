use crate::*;

#[derive(Clone, Default)]
/// Workspace for QBVH update.
///
/// Re-using the same workspace for multiple QBVH modification isn’t mandatory, but it improves
/// performances by avoiding useless allocation.
pub struct QbvhUpdateWorkspace<BV> {
  // node id --> node depth
  stack: Vec<(u32, u32)>,
  pub(crate) orig_ids: Vec<u32>,
  pub(crate) is_leaf: Vec<bool>,
  pub(crate) aabbs: Vec<BV>,
  pub(crate) margins: Vec<f32>,
}

impl<B> QbvhUpdateWorkspace<B> {
  fn clear(&mut self) {
    self.stack.clear();
    self.orig_ids.clear();
    self.is_leaf.clear();
    self.aabbs.clear();
    self.margins.clear();
  }
}

impl<LeafData: IndexedData, BV, SimdBV> Qbvh<LeafData, BV, SimdBV> {
  /// Immediately remove a leaf from this QBVH.
  pub fn remove(&mut self, data: LeafData) -> Option<LeafData> {
    let id = data.index();
    let proxy = self.proxies.get_mut(id)?;
    let node = self.nodes.get_mut(proxy.node.index as usize)?;
    node.children[proxy.node.lane as usize] = u32::MAX;

    if !node.is_dirty() {
      node.set_dirty(true);
      self.dirty_bounding_nodes.push(proxy.node.index);
    }

    *proxy = QbvhProxy::invalid();

    Some(data)
  }

  /// Prepare to update the margin of the node attached by given leaf data. Do nothing
  /// if the leaf is detached. The margin will be updated after the next call to refit_margin.
  pub fn pre_update_margin(&mut self, data: LeafData) {
    let proxy_id = data.index();
    if let Some(proxy) = self.proxies.get_mut(proxy_id) {
      proxy.data = data;

      if proxy.is_detached() {
        return;
      }

      let node = &mut self.nodes[proxy.node.index as usize];
      if !node.is_margin_dirty() {
        node.set_margin_dirty(true);
        self.dirty_margin_nodes.push(proxy.node.index);
      }
    }
  }

  /// Prepare a new leaf for insertion into this QBVH (or for update if it already exists).
  /// The insertion or update will be completely valid only after the next call to [`Qbvh::refit`].
  pub fn pre_update_bounding_or_insert(&mut self, data: LeafData)
  where
    BV: Default,
    SimdBV: SimdValue<Element = BV> + From<[BV; QBVH_SIMD_WIDTH]> + Copy,
  {
    if self.nodes.is_empty() {
      let mut root = QbvhNode::empty();
      root.children = [1, u32::MAX, u32::MAX, u32::MAX];
      self
        .nodes
        .extend_from_slice(&[root, QbvhNode::empty_leaf_with_parent(NodeIndex::new(0, 0))]);
    }

    let proxy_id = data.index();

    if self.proxies.len() <= proxy_id {
      self.proxies.resize(proxy_id + 1, QbvhProxy::invalid());
    }

    let proxy = &mut self.proxies[proxy_id];
    proxy.data = data;

    if proxy.is_detached() {
      // Attach the proxy into one of the leaf attached to the root, if there is room.
      for lane in 0..QBVH_SIMD_WIDTH {
        let mut child = self.nodes[0].children[lane];

        if child == u32::MAX {
          // Missing node, create it.
          child = self.nodes.len() as u32;
          self
            .nodes
            .push(QbvhNode::empty_leaf_with_parent(NodeIndex::new(
              0, lane as u8,
            )));
          self.nodes[0].children[lane] = child;
        }

        let child_node = &mut self.nodes[child as usize];
        // skip non-leaf node that is attached to the root
        if !child_node.is_leaf() {
          continue;
        }

        // Insert into this node if there is room.
        for lane in 0..QBVH_SIMD_WIDTH {
          if child_node.children[lane] == u32::MAX {
            child_node.children[lane] = proxy_id as u32;
            proxy.node = NodeIndex::new(child, lane as u8);

            if !child_node.is_dirty() {
              self.dirty_bounding_nodes.push(child);
              child_node.set_dirty(true);
            }

            return;
          }
        }
      }

      // If we reached this point, we didn’t find room for our
      // new proxy. Create a new root to make more room.
      let mut old_root = self.nodes[0];
      old_root.parent = NodeIndex::new(0, 0);

      for child_id in old_root.children {
        let parent_index = self.nodes.len() as u32;

        if let Some(child_node) = self.nodes.get_mut(child_id as usize) {
          child_node.parent.index = parent_index;
        }
      }

      let new_leaf_node_id = self.nodes.len() as u32 + 1;
      let mut new_leaf_node = QbvhNode::empty_leaf_with_parent(NodeIndex::new(0, 1));
      new_leaf_node.children[0] = proxy_id as u32;
      // The first new node contains the (unmodified) old root, the second
      // new node contains the newly added proxy.
      self.dirty_bounding_nodes.push(new_leaf_node_id);
      new_leaf_node.set_dirty(true);
      proxy.node = NodeIndex::new(new_leaf_node_id, 0);

      let new_root_children = [
        self.nodes.len() as u32,
        new_leaf_node_id,
        u32::MAX,
        u32::MAX,
      ];

      self.nodes.extend_from_slice(&[old_root, new_leaf_node]);
      self.nodes[0].children = new_root_children;
    } else {
      let node = &mut self.nodes[proxy.node.index as usize];
      if !node.is_dirty() {
        node.set_dirty(true);
        self.dirty_bounding_nodes.push(proxy.node.index);
      }
    }
  }

  /// Update all the nodes that have been marked as dirty by [`Qbvh::pre_update_margin`],
  /// this call will never cause node bounding changed.
  pub fn refit_margin<F>(&mut self, margin_builder: F)
  where
    F: Fn(&LeafData) -> f32,
  {
    let mut dirty_parent_nodes: Vec<u32> = Default::default();
    while !self.dirty_margin_nodes.is_empty() {
      while let Some(id) = self.dirty_margin_nodes.pop() {
        if let Some(node) = self.nodes.get(id as usize) {
          let mut new_margins = [0.; QBVH_SIMD_WIDTH];
          for (child_id, new_margin) in node.children.iter().zip(new_margins.iter_mut()) {
            if node.is_leaf() {
              if let Some(proxy) = self.proxies.get(*child_id as usize) {
                *new_margin = margin_builder(&proxy.data);
              }
            } else if let Some(node) = self.nodes.get(*child_id as usize) {
              *new_margin = node.simd_margin.simd_horizontal_max();
            }
          }

          let node = &mut self.nodes[id as usize];
          node.set_margin_dirty(false);
          node.simd_margin = new_margins.into();

          let parent_id = node.parent.index;
          if let Some(parent) = self.nodes.get_mut(parent_id as usize) {
            if !parent.is_margin_dirty() {
              dirty_parent_nodes.push(parent_id);
              parent.set_margin_dirty(true);
            }
          }
        }
      }
      std::mem::swap(&mut self.dirty_margin_nodes, &mut dirty_parent_nodes);
    }
  }

  /// Update all the nodes that have been marked as dirty by [`Qbvh::pre_update_or_insert`],
  /// and [`Qbvh::remove`].
  ///
  /// This will not alter the topology of this `Qbvh`.
  pub fn refit_bounding<F>(&mut self, aabb_builder: F) -> usize
  where
    BV: From<SimdBV>,
    SimdBV: SimdValue<Element = BV> + From<[BV; QBVH_SIMD_WIDTH]> + Copy,
    F: Fn(&LeafData) -> BV,
  {
    // Loop on the dirty leaves.
    let mut dirty_parent_nodes: Vec<u32> = Default::default();
    let mut num_changed = 0;

    while !self.dirty_bounding_nodes.is_empty() {
      while let Some(id) = self.dirty_bounding_nodes.pop() {
        // NOTE: this will cover the case where we reach the root of the tree.
        if let Some(node) = self.nodes.get(id as usize) {
          // Compute the new aabb.
          let mut new_aabbs = array!(|lane| node.simd_aabb.extract(lane));

          for (child_id, new_aabb) in node.children.iter().zip(new_aabbs.iter_mut()) {
            if node.is_leaf() {
              // We are in a leaf: compute the Aabbs.
              if let Some(proxy) = self.proxies.get(*child_id as usize) {
                *new_aabb = aabb_builder(&proxy.data);
              }
            } else {
              // We are in an internal node: compute the children's Aabbs.
              if let Some(node) = self.nodes.get(*child_id as usize) {
                *new_aabb = node.simd_aabb.into();
              }
            }
          }

          let node = &mut self.nodes[id as usize];
          node.set_dirty(false);
          node.set_changed(true);
          node.simd_aabb = new_aabbs.into();
          num_changed += 1;

          let parent_id = node.parent.index;
          if let Some(parent) = self.nodes.get_mut(parent_id as usize) {
            if !parent.is_dirty() {
              dirty_parent_nodes.push(parent_id);
              parent.set_dirty(true);
            }
          }
        }
      }
      std::mem::swap(&mut self.dirty_bounding_nodes, &mut dirty_parent_nodes);
    }

    num_changed
  }
}

impl<LeafData: IndexedData, BV, SimdBV> Qbvh<LeafData, BV, SimdBV>
where
  BV: Default + From<SimdBV> + Copy,
  SimdBV: SimdValue<Element = BV> + From<[BV; QBVH_SIMD_WIDTH]> + Copy,
{
  /// Rebalances the `Qbvh` tree.
  ///
  /// This will modify the topology of this tree. This assumes that the leaf AABBs have
  /// already been updated with [`Qbvh::refit`].
  pub fn rebalance(
    &mut self,
    workspace: &mut QbvhUpdateWorkspace<BV>,
    mut splitter: impl QbvhDataSplitter<BV>,
  ) {
    if self.nodes.is_empty() {
      return;
    }

    workspace.clear();

    const MIN_CHANGED_DEPTH: u32 = 5; // TODO: find a good value

    for root_child in self.nodes[0].children {
      if root_child < self.nodes.len() as u32 {
        workspace.stack.push((root_child, 1));
      }
    }

    // Collect empty slots and pseudo-leaves to sort.
    while let Some((id, depth)) = workspace.stack.pop() {
      let node = &mut self.nodes[id as usize];

      if node.is_leaf() {
        self.free_list.push(id);
        for lane in 0..QBVH_SIMD_WIDTH {
          let proxy_id = node.children[lane];
          if proxy_id < self.proxies.len() as u32 {
            workspace.orig_ids.push(proxy_id);
            workspace.aabbs.push(node.simd_aabb.extract(lane));
            workspace.margins.push(node.simd_margin.extract(lane));
            workspace.is_leaf.push(true);
          }
        }
      } else if node.is_changed() || depth < MIN_CHANGED_DEPTH {
        self.free_list.push(id);

        for child in node.children {
          if child < self.nodes.len() as u32 {
            workspace.stack.push((child, depth + 1));
          }
        }
      } else {
        workspace.orig_ids.push(id);
        workspace.aabbs.push(node.simd_aabb.into());
        workspace
          .margins
          .push(node.simd_margin.simd_horizontal_max());
        workspace.is_leaf.push(false);
      }
    }

    let root_id = NodeIndex::new(0, 0);
    let mut indices = Vec::from_iter(0..workspace.orig_ids.len());
    let (id, aabb, margin) =
      self.do_recurse_rebalance(&mut indices, workspace, root_id, &mut splitter);

    self.root_aabb = aabb;
    self.nodes[0] = QbvhNode {
      simd_aabb: [aabb, BV::default(), BV::default(), BV::default()].into(),
      simd_margin: [margin, 0., 0., 0.].into(),
      children: [id, u32::MAX, u32::MAX, u32::MAX],
      parent: NodeIndex::invalid(),
      flags: QbvhNodeFlags::default(),
    };
  }

  pub(crate) fn do_recurse_rebalance(
    &mut self,
    indices: &mut [usize],
    workspace: &QbvhUpdateWorkspace<BV>,
    parent: NodeIndex,
    splitter: &mut impl QbvhDataSplitter<BV>,
  ) -> (u32, BV, f32) {
    if indices.len() <= 4 {
      return self.rebalance_less_than_4(parent, indices, workspace);
    }

    let node = QbvhNode {
      simd_aabb: SimdBV::splat(BV::default()),
      simd_margin: SimdRealValue::zero(),
      children: [0; 4], // Will be set after the recursive call
      parent,
      flags: QbvhNodeFlags::default(),
    };

    let nid = if let Some(nid) = self.free_list.pop() {
      self.nodes[nid as usize] = node;
      nid
    } else {
      let nid = self.nodes.len();
      self.nodes.push(node);
      nid as u32
    };

    let splits = splitter.split_dataset(indices, &workspace.aabbs);
    let n = [
      NodeIndex::new(nid, 0),
      NodeIndex::new(nid, 1),
      NodeIndex::new(nid, 2),
      NodeIndex::new(nid, 3),
    ];

    let children = [
      self.do_recurse_rebalance(splits[0], workspace, n[0], splitter),
      self.do_recurse_rebalance(splits[1], workspace, n[1], splitter),
      self.do_recurse_rebalance(splits[2], workspace, n[2], splitter),
      self.do_recurse_rebalance(splits[3], workspace, n[3], splitter),
    ];

    // Now we know the indices of the child nodes.
    self.nodes[nid as usize].children =
      [children[0].0, children[1].0, children[2].0, children[3].0];
    self.nodes[nid as usize].simd_aabb =
      [children[0].1, children[1].1, children[2].1, children[3].1].into();
    self.nodes[nid as usize].simd_margin =
      [children[0].2, children[1].2, children[2].2, children[3].2].into();

    let my_aabb = self.nodes[nid as usize].simd_aabb.into();
    let my_aabb_margin = self.nodes[nid as usize].simd_margin.simd_horizontal_max();
    (nid, my_aabb, my_aabb_margin)
  }

  fn rebalance_less_than_4(
    &mut self,
    parent: NodeIndex,
    indices: &mut [usize],
    workspace: &QbvhUpdateWorkspace<BV>,
  ) -> (u32, BV, f32) {
    // Leaf case.
    let mut has_leaf = false;
    let mut has_internal = false;

    for id in &*indices {
      has_leaf |= workspace.is_leaf[*id];
      has_internal |= !workspace.is_leaf[*id];
    }

    let my_internal_id = if has_internal {
      self.free_list.pop().unwrap_or_else(|| {
        self.nodes.push(QbvhNode::empty());
        self.nodes.len() as u32 - 1
      })
    } else {
      u32::MAX
    };

    let my_leaf_id = if has_leaf {
      self.free_list.pop().unwrap_or_else(|| {
        self.nodes.push(QbvhNode::empty());
        self.nodes.len() as u32 - 1
      })
    } else {
      u32::MAX
    };

    let mut leaf_aabbs = [BV::default(); 4];
    let mut internal_aabbs = [BV::default(); 4];

    let mut leaf_aabb_margins = [0_f32; 4];
    let mut internal_aabb_margins = [0_f32; 4];

    let mut proxy_ids = [u32::MAX; 4];
    let mut internal_ids = [u32::MAX; 4];
    let mut new_internal_lane_containing_leaf = usize::MAX;

    for (k, id) in indices.iter().enumerate() {
      let orig_id = workspace.orig_ids[*id];
      if workspace.is_leaf[*id] {
        new_internal_lane_containing_leaf = k;
        leaf_aabbs[k] = workspace.aabbs[*id];
        leaf_aabb_margins[k] = workspace.margins[*id];

        proxy_ids[k] = orig_id;
        self.proxies[orig_id as usize].node = NodeIndex::new(my_leaf_id, k as u8);
      } else {
        internal_aabbs[k] = workspace.aabbs[*id];
        internal_aabb_margins[k] = workspace.margins[*id];

        internal_ids[k] = orig_id;
        self.nodes[orig_id as usize].parent = NodeIndex::new(my_internal_id, k as u8);
      }
    }

    let leaf_simd_aabbs = SimdBV::from(leaf_aabbs);
    let union_leaf_aabb = BV::from(leaf_simd_aabbs);
    let leaf_simd_margins = SimdRealValue::from(leaf_aabb_margins);
    let max_leaf_margin = leaf_simd_margins.simd_horizontal_max();

    if has_internal && has_leaf {
      internal_ids[new_internal_lane_containing_leaf] = my_leaf_id;
      internal_aabbs[new_internal_lane_containing_leaf] = union_leaf_aabb;
      internal_aabb_margins[new_internal_lane_containing_leaf] = max_leaf_margin;
    }

    if has_leaf {
      let leaf_node = QbvhNode {
        simd_aabb: leaf_simd_aabbs,
        simd_margin: leaf_aabb_margins.into(),
        children: proxy_ids,
        parent: if has_internal {
          NodeIndex::new(my_internal_id, new_internal_lane_containing_leaf as u8)
        } else {
          parent
        },
        flags: QbvhNodeFlags::LEAF,
      };
      self.nodes[my_leaf_id as usize] = leaf_node;
    }

    if has_internal {
      let simd_aabb = SimdBV::from(internal_aabbs);
      let union_internal_aabb = BV::from(simd_aabb);
      let simd_margin = SimdRealValue::from(internal_aabb_margins);
      let max_internal_margin = simd_margin.simd_horizontal_max();
      let internal_node = QbvhNode {
        simd_aabb,
        simd_margin,
        children: internal_ids,
        parent,
        flags: QbvhNodeFlags::default(),
      };
      self.nodes[my_internal_id as usize] = internal_node;
      (my_internal_id, union_internal_aabb, max_internal_margin)
    } else {
      (my_leaf_id, union_leaf_aabb, max_leaf_margin)
    }
  }
}
