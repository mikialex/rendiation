use crate::*;

/// Trait used for generating the content of the leaves of the Qbvh acceleration structure.
pub trait QbvhDataGenerator<LeafData, BV> {
  /// Gives an idea of the number of elements this generator contains.
  ///
  /// This is primarily used for pre-allocating some arrays for better performances.
  fn size_hint(&self) -> usize;
  /// Iterate through all the elements of this generator.
  fn for_each(&mut self, f: impl FnMut(LeafData, BV));
}

impl<LeafData, BV, F> QbvhDataGenerator<LeafData, BV> for F
where
  F: ExactSizeIterator<Item = (LeafData, BV)>,
{
  fn size_hint(&self) -> usize {
    self.len()
  }

  #[inline(always)]
  fn for_each(&mut self, mut f: impl FnMut(LeafData, BV)) {
    for (elt, aabb) in self {
      f(elt, aabb)
    }
  }
}

impl<LeafData: IndexedData, BV, SimdBV> Qbvh<LeafData, BV, SimdBV>
where
  BV: Default + From<SimdBV> + Copy,
  SimdBV: SimdValue<Element = BV> + From<[BV; QBVH_SIMD_WIDTH]> + Copy,
{
  pub fn clear_and_rebuild(
    &mut self,
    mut data_gen: impl QbvhDataGenerator<LeafData, BV>,
    mut splitter: impl QbvhDataSplitter<BV>,
  ) {
    self.free_list.clear();
    self.nodes.clear();
    self.dirty_bounding_nodes.clear();
    self.proxies.clear();

    // Create proxies.
    let mut work_space = QbvhUpdateWorkspace::default();
    work_space.orig_ids = Vec::with_capacity(data_gen.size_hint());
    work_space.aabbs = Vec::with_capacity(data_gen.size_hint());
    work_space.margins = Vec::with_capacity(data_gen.size_hint());
    work_space.is_leaf = vec![true; data_gen.size_hint()];
    self.proxies = vec![QbvhProxy::invalid(); data_gen.size_hint()];

    data_gen.for_each(|data, aabb| {
      let index = data.index();
      if index >= self.proxies.len() {
        self.proxies.resize(index + 1, QbvhProxy::invalid());
        work_space.is_leaf.resize(index + 1, true);
      }

      self.proxies[index].data = data;
      work_space.aabbs.push(aabb);
      work_space.margins.push(0.);
      work_space.orig_ids.push(index as u32);
    });

    // Build the tree recursively.
    let root_node = QbvhNode {
      simd_aabb: SimdBV::splat(BV::default()),
      simd_margin: SimdRealValue::zero(),
      children: [1, u32::MAX, u32::MAX, u32::MAX],
      parent: NodeIndex::invalid(),
      flags: QbvhNodeFlags::default(),
    };

    self.nodes.push(root_node);
    let root_id = NodeIndex::new(0, 0);
    let mut indices = Vec::from_iter(0..work_space.orig_ids.len());
    let (_, aabb, margin) =
      self.do_recurse_rebalance(&mut indices, &work_space, root_id, &mut splitter);

    self.root_aabb = aabb;
    self.nodes[0].simd_aabb = SimdBV::from([aabb, BV::default(), BV::default(), BV::default()]);
    self.nodes[0].simd_margin = [margin, 0., 0., 0.].into();
  }
}
