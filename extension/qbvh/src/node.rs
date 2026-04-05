use crate::*;

/// The index of one specific node of a Qbvh.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NodeIndex {
  /// The index of the SIMD node containing the addressed node.
  pub index: u32, // Index of the addressed node in the `nodes` array.
  /// The SIMD lane the addressed node is associated to.
  pub lane: u8, // SIMD lane of the addressed node.
}

impl NodeIndex {
  pub(crate) fn new(index: u32, lane: u8) -> Self {
    Self { index, lane }
  }

  pub(crate) fn invalid() -> Self {
    Self {
      index: u32::MAX,
      lane: 0,
    }
  }

  pub(crate) fn is_invalid(&self) -> bool {
    self.index == u32::MAX
  }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Default, Debug)]
    /// The status of a QBVH node.
    pub struct QbvhNodeFlags: u8 {
        /// If this bit is set, the node is a leaf.
        const LEAF = 0b0001;
        /// If this bit is set, this node was recently changed.
        const CHANGED = 0b0010;
        /// Does this node need an update?
        const DIRTY = 0b0100;
        const MARGIN = 0b1000;
    }
}

/// A SIMD node of an SIMD Qbvh.
///
/// This groups four nodes of the Qbvh.
#[derive(Copy, Clone, Debug)]
pub struct QbvhNode<SimdBV> {
  /// The Aabbs of the qbvh nodes represented by this node.
  pub simd_aabb: SimdBV,
  /// The margin for each box,
  pub simd_margin: SimdRealValue,
  /// Index of the nodes of the 4 nodes represented by `self`.
  /// If this is a leaf, it contains the proxy ids instead.
  pub children: [u32; 4],
  /// The index of the node parent to the 4 nodes represented by `self`.
  pub parent: NodeIndex,
  /// Status flags for this node.
  pub flags: QbvhNodeFlags,
}

impl<SimdBV> QbvhNode<SimdBV> {
  #[inline]
  /// Is this node a leaf?
  pub fn is_leaf(&self) -> bool {
    self.flags.contains(QbvhNodeFlags::LEAF)
  }

  #[inline]
  /// Does the AABB of this node needs to be updated?
  pub fn is_dirty(&self) -> bool {
    self.flags.contains(QbvhNodeFlags::DIRTY)
  }

  #[inline]
  /// Does the margin of this node needs to be updated?
  pub fn is_margin_dirty(&self) -> bool {
    self.flags.contains(QbvhNodeFlags::MARGIN)
  }

  #[inline]
  /// Sets if the AABB of this node needs to be updated.
  pub fn set_dirty(&mut self, dirty: bool) {
    self.flags.set(QbvhNodeFlags::DIRTY, dirty);
  }

  #[inline]
  /// Sets if the margin of this node needs to be updated.
  pub fn set_margin_dirty(&mut self, dirty: bool) {
    self.flags.set(QbvhNodeFlags::MARGIN, dirty);
  }

  #[inline]
  /// Was the AABB of this node changed since the last rebalancing?
  pub fn is_changed(&self) -> bool {
    self.flags.contains(QbvhNodeFlags::CHANGED)
  }

  #[inline]
  /// Sets if the AABB of this node changed since the last rebalancing.
  pub fn set_changed(&mut self, changed: bool) {
    self.flags.set(QbvhNodeFlags::CHANGED, changed);
  }
}

impl<B: Default, SimdBV: SimdValue<Element = B>> QbvhNode<SimdBV> {
  #[inline]
  /// An empty internal node.
  pub fn empty() -> Self {
    Self {
      simd_aabb: SimdBV::splat(B::default()),
      simd_margin: SimdRealValue::zero(),
      children: [u32::MAX; 4],
      parent: NodeIndex::invalid(),
      flags: QbvhNodeFlags::default(),
    }
  }

  #[inline]
  /// An empty leaf.
  pub fn empty_leaf_with_parent(parent: NodeIndex) -> Self {
    Self {
      simd_aabb: SimdBV::splat(B::default()),
      simd_margin: SimdRealValue::zero(),
      children: [u32::MAX; 4],
      parent,
      flags: QbvhNodeFlags::LEAF,
    }
  }
}
