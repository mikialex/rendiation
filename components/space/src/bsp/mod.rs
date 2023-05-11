/// https://github.com/kvark/binary-space-partition/blob/master/src/lib.rs
///
/// Binary Space Partitioning (BSP)
/// Provides an abstract `BspNode` structure, which can be seen as a tree.
/// Useful for quickly ordering polygons along a particular view vector.
/// Is not tied to a particular math library.
use crate::*;

pub mod csg;

impl<T> AbstractTreeNode for BspNode<T> {
  fn visit_children(&self, mut visitor: impl FnMut(&Self)) {
    if let Some(n) = self.front.as_ref() {
      visitor(n.as_ref())
    }
    if let Some(n) = self.back.as_ref() {
      visitor(n.as_ref())
    }
  }
}

impl<T> AbstractTreeMutNode for BspNode<T> {
  fn visit_children_mut(&mut self, mut visitor: impl FnMut(&mut Self)) {
    if let Some(n) = self.front.as_mut() {
      visitor(n.as_mut())
    }
    if let Some(n) = self.back.as_mut() {
      visitor(n.as_mut())
    }
  }
}

/// A plane abstracted to the matter of partitioning.
pub trait BSPPlane: Sized + Clone {
  type PlaneCut: PlaneCutResult<Self>;

  /// Try to cut a different plane by this one.
  fn cut(&self, plane: &Self) -> Self::PlaneCut;
}

pub trait BSPPlaneAlignable {
  /// Check if a different plane is aligned in the same direction
  /// as this one.
  fn is_aligned(&self, plane: &Self) -> bool;
}

/// Use this trait as the abstraction of the cutting result
/// is because we want avoid any allocation at best when get the front and back result.
pub trait PlaneCutResult<T> {
  /// If the current plane is exact same plane, which actually not been cut.
  /// If false, we check the front and back result.
  fn is_sibling(&self) -> bool;
  fn iter_front(&self, visitor: impl FnMut(T));
  fn iter_back(&self, visitor: impl FnMut(T));
}

/// A node in the `BspTree`, which can be considered a tree itself.
#[derive(Clone, Debug)]
pub struct BspNode<T> {
  coplanar: Vec<T>,
  front: Option<Box<BspNode<T>>>,
  back: Option<Box<BspNode<T>>>,
}

impl<T> BspNode<T> {
  /// Create a new node.
  pub fn new() -> Self {
    BspNode {
      coplanar: Vec::new(),
      front: None,
      back: None,
    }
  }
}

impl<T> Default for BspNode<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T: BSPPlane> BspNode<T> {
  /// Insert a value into the sub-tree starting with this node.
  /// This operation may spawn additional leafs/branches of the tree.
  pub fn insert(&mut self, value: T) {
    if let Some(first) = self.coplanar.first() {
      let cut_result = first.cut(&value);
      if cut_result.is_sibling() {
        self.coplanar.push(value)
      } else {
        cut_result.iter_front(|p| self.front.get_or_insert_default().insert(p));
        cut_result.iter_back(|p| self.back.get_or_insert_default().insert(p));
      }
    } else {
      self.coplanar.push(value);
    }
  }

  /// Build the draw order of this sub-tree into an `out` vector,
  /// so that the contained planes are sorted back to front according
  /// to the view vector defined as the `base` plane front direction.
  pub fn order(&self, base: &T, out: &mut Vec<T>)
  where
    T: BSPPlaneAlignable,
  {
    let (former, latter) = match self.coplanar.first() {
      None => return,
      Some(first) if base.is_aligned(first) => (self.front.as_ref(), self.back.as_ref()),
      Some(_) => (self.back.as_ref(), self.front.as_ref()),
    };

    if let Some(node) = former {
      node.order(base, out);
    }

    out.extend_from_slice(&self.coplanar);

    if let Some(node) = latter {
      node.order(base, out);
    }
  }
}
