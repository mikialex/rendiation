use crate::*;

/// The status of the spatial partitioning structure traversal.
pub enum SimdVisitStatus {
  /// The traversal should continue on the children of the currently visited nodes for which
  /// the boolean lane is set to `1`.
  MaybeContinue(SimdBoolValue),
  /// The traversal should exit immediately.
  ExitEarly,
}

pub type MayBeDataGroup<T> = Option<[Option<T>; SIMD_WIDTH]>;
/// Trait implemented by visitor called during the traversal of a spatial partitioning data structure.
pub trait SimdVisitor<LeafData, SimdBV, Margin> {
  /// Execute an operation on the content of a node of the spatial partitioning structure.
  ///
  /// Returns whether the traversal should continue on the node's children, if it should not continue
  /// on those children, or if the whole traversal should be exited early.
  fn visit(
    &mut self,
    id: u32,
    bv: &SimdBV,
    margin: &Margin,
    data: MayBeDataGroup<&LeafData>,
  ) -> SimdVisitStatus;
}

impl<LeafData, SimdBV, Margin, F> SimdVisitor<LeafData, SimdBV, Margin> for F
where
  F: FnMut(u32, &SimdBV, &Margin, MayBeDataGroup<&LeafData>) -> SimdVisitStatus,
{
  fn visit(
    &mut self,
    id: u32,
    bv: &SimdBV,
    margin: &Margin,
    data: MayBeDataGroup<&LeafData>,
  ) -> SimdVisitStatus {
    (self)(id, bv, margin, data)
  }
}

/// The next action to be taken by a BVH traversal algorithm after having visited a node with some data.
pub enum SimdBestFirstVisitStatus<Res> {
  /// The traversal can continue.
  MaybeContinue {
    /// The weight associated to each child of the node being traversed.
    weights: SimdRealValue,
    /// Each lane indicates if the corresponding child of the node being traversed
    /// should be traversed too.
    mask: SimdBoolValue,
    /// Optional results associated to each child of the node being traversed.
    results: [Option<Res>; SIMD_WIDTH],
  },
  /// The traversal aborts.
  ///
  /// If a data is provided, then it is returned as the result of the traversal.
  /// If no result is provided, then the last best result found becomes the result of the traversal.
  ExitEarly(Option<Res>),
}

/// Trait implemented by cost functions used by the best-first search on a `BVT`.
pub trait SimdBestFirstVisitor<LeafData, SimdBV, Margin> {
  /// The result of a best-first traversal.
  type Result;

  /// Compute the next action to be taken by the best-first-search after visiting a node containing the given bounding volume.
  fn visit(
    &mut self,
    best_cost_so_far: f32,
    bv: &SimdBV,
    margin: &Margin,
    value: Option<[Option<&LeafData>; SIMD_WIDTH]>,
  ) -> SimdBestFirstVisitStatus<Self::Result>;
}

#[derive(Copy, Clone)]
pub struct WeightedValue<T> {
  pub value: T,
  pub cost: f32,
}

impl<T> WeightedValue<T> {
  /// Creates a new reference packed with a cost value.
  #[inline]
  pub fn new(value: T, cost: f32) -> WeightedValue<T> {
    WeightedValue { value, cost }
  }
}

impl<T> PartialEq for WeightedValue<T> {
  #[inline]
  fn eq(&self, other: &WeightedValue<T>) -> bool {
    self.cost.eq(&other.cost)
  }
}

impl<T> Eq for WeightedValue<T> {}

impl<T> PartialOrd for WeightedValue<T> {
  #[inline]
  fn partial_cmp(&self, other: &WeightedValue<T>) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl<T> Ord for WeightedValue<T> {
  #[inline]
  fn cmp(&self, other: &WeightedValue<T>) -> Ordering {
    if self.cost < other.cost {
      Ordering::Less
    } else if self.cost > other.cost {
      Ordering::Greater
    } else {
      Ordering::Equal
    }
  }
}
