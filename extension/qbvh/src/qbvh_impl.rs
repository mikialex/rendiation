use crate::*;

/// A data to which an index is associated.
pub trait IndexedData: Copy {
  /// Creates a new default instance of `Self`.
  fn default() -> Self;
  /// Gets the index associated to `self`.
  fn index(&self) -> usize;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// Combination of a leaf data and its associated node’s index.
pub struct QbvhProxy<LeafData> {
  /// Index of the leaf node the leaf data is associated to.
  pub node: NodeIndex,
  /// The data contained in this node.
  pub data: LeafData,
}

impl<LeafData> QbvhProxy<LeafData> {
  pub(crate) fn invalid() -> Self
  where
    LeafData: IndexedData,
  {
    Self {
      node: NodeIndex::invalid(),
      data: LeafData::default(),
    }
  }

  pub(crate) fn is_detached(&self) -> bool {
    self.node.is_invalid()
  }
}

// TODO: replace allocator for nodes?
#[derive(Debug, Clone)]
pub struct Qbvh<LeafData, BV, SimdBV> {
  pub(crate) root_aabb: BV,
  pub(crate) nodes: Vec<QbvhNode<SimdBV>>,
  pub(crate) dirty_bounding_nodes: Vec<u32>,
  pub(crate) dirty_margin_nodes: Vec<u32>,
  pub(crate) free_list: Vec<u32>,
  // todo, considering sparse mapping
  pub(crate) proxies: Vec<QbvhProxy<LeafData>>,
}

impl<LeafData, BV: Default, SimdBV> Default for Qbvh<LeafData, BV, SimdBV> {
  fn default() -> Self {
    Qbvh {
      root_aabb: BV::default(),
      nodes: Vec::new(),
      dirty_bounding_nodes: Vec::new(),
      dirty_margin_nodes: Vec::new(),
      free_list: Vec::new(),
      proxies: Vec::new(),
    }
  }
}

impl<LeafData, BV, SimdBV> Qbvh<LeafData, BV, SimdBV>
where
  SimdBV: SimdValue<Element = BV>,
{
  /// The Aabb of the given node.
  pub fn node_aabb(&self, node_id: NodeIndex) -> Option<BV> {
    self
      .nodes
      .get(node_id.index as usize)
      .map(|n| n.simd_aabb.extract(node_id.lane as usize))
  }
}

impl<LeafData: IndexedData, BV, SimdBV> Qbvh<LeafData, BV, SimdBV> {
  /// The Aabb of the root of this tree.
  pub fn root_aabb(&self) -> &BV {
    &self.root_aabb
  }
  /// The margin of the root of this tree.
  pub fn root_margin(&self) -> f32 {
    if self.nodes.is_empty() {
      return 0.;
    }
    self.nodes[0].simd_margin.extract(0)
  }

  /// Iterates mutably through all the leaf data in this Qbvh.
  pub fn iter_data_mut(&mut self) -> impl Iterator<Item = (NodeIndex, &mut LeafData)> {
    self.proxies.iter_mut().map(|p| (p.node, &mut p.data))
  }

  /// Iterate through all the leaf data in this Qbvh.
  pub fn iter_data(&self) -> impl Iterator<Item = (NodeIndex, &LeafData)> {
    self.proxies.iter().map(|p| (p.node, &p.data))
  }

  pub fn create_node_ref(&self, id: usize) -> QbvhTreeNodeRef<'_, LeafData, BV, SimdBV> {
    QbvhTreeNodeRef {
      node: &self.nodes[id],
      qbvh: self,
    }
  }

  /// Returns the data associated to a given leaf.
  ///
  /// Returns `None` if the provided node ID does not identify a leaf.
  pub fn leaf_data(&self, node_id: NodeIndex) -> Option<LeafData> {
    let node = self.nodes.get(node_id.index as usize)?;

    if !node.is_leaf() {
      return None;
    }

    let proxy = self
      .proxies
      .get(node.children[node_id.lane as usize] as usize)?;
    Some(proxy.data)
  }

  // check index is node
  pub fn node_get_leafs_data(&self, index: u32) -> Vec<LeafData> {
    let len = self.nodes.len();
    let mut stack = vec![index];
    let mut leaf_cache = vec![];

    while let Some(index) = stack.pop() {
      let node = &self.nodes[index as usize];

      if node.is_leaf() {
        for lane in 0..QBVH_SIMD_WIDTH {
          if let Some(leaf_data) = self.proxies.get(node.children[lane] as usize) {
            leaf_cache.push(leaf_data.data);
          }
        }
      } else {
        for lane in 0..QBVH_SIMD_WIDTH {
          if node.children[lane] as usize <= len {
            stack.push(node.children[lane]);
          }
        }
      }
    }
    leaf_cache
  }

  pub fn check_topology(&self) {
    if self.nodes.is_empty() {
      return;
    }

    let mut stack = vec![];
    stack.push(0);

    let mut node_id_found = vec![false; self.nodes.len()];
    let mut proxy_id_found = vec![false; self.proxies.len()];

    while let Some(id) = stack.pop() {
      let node = &self.nodes[id as usize];

      // No duplicate references to internal nodes.
      assert!(!node_id_found[id as usize]);
      node_id_found[id as usize] = true;

      if id != 0 {
        let parent = &self.nodes[node.parent.index as usize];

        // Check parent <-> children indices.
        assert!(!parent.is_leaf());
        assert_eq!(parent.children[node.parent.lane as usize], id);

        // If this node is changed, its parent is changed too.
        if node.is_changed() {
          assert!(parent.is_changed());
        }
      }

      if node.is_leaf() {
        for ii in 0..QBVH_SIMD_WIDTH {
          let proxy_id = node.children[ii];
          if proxy_id != u32::MAX {
            // No duplicate references to proxies.
            assert!(!proxy_id_found[proxy_id as usize]);
            proxy_id_found[proxy_id as usize] = true;

            // Proxy node reference is correct.
            assert_eq!(
              self.proxies[proxy_id as usize].node,
              NodeIndex::new(id, ii as u8),
              "Failed {proxy_id}"
            );
          }
        }
      } else {
        for child in node.children {
          if child != u32::MAX {
            stack.push(child);
          }
        }
      }
    }

    // Each proxy was visited once.
    assert_eq!(
      proxy_id_found.iter().filter(|found| **found).count(),
      self.proxies.iter().filter(|p| !p.is_detached()).count()
    );
  }
}

impl IndexedData for usize {
  fn default() -> Self {
    u32::MAX as usize
  }
  fn index(&self) -> usize {
    *self
  }
}

impl IndexedData for u32 {
  fn default() -> Self {
    u32::MAX
  }

  fn index(&self) -> usize {
    *self as usize
  }
}

pub struct QbvhTreeNodeRef<'a, Leaf, BV, SimdBV> {
  node: &'a QbvhNode<SimdBV>,
  qbvh: &'a Qbvh<Leaf, BV, SimdBV>,
}

impl<'a, Leaf: IndexedData, BV, SimdBV> AbstractTreeNode for QbvhTreeNodeRef<'a, Leaf, BV, SimdBV> {
  fn visit_children(&self, mut visitor: impl FnMut(&Self)) {
    if !self.node.is_leaf() {
      for lane in 0..QBVH_SIMD_WIDTH {
        let entry = self.node.children[lane] as usize;
        if entry <= self.qbvh.nodes.len() {
          visitor(&self.qbvh.create_node_ref(entry));
        }
      }
    }
  }
}
