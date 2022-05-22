/// https://github.com/kvark/binary-space-partition/blob/master/src/lib.rs
///
/// Binary Space Partitioning (BSP)
/// Provides an abstract `BspNode` structure, which can be seen as a tree.
/// Useful for quickly ordering polygons along a particular view vector.
/// Is not tied to a particular math library.
///

/// The result of one plane being cut by another one.
/// The "cut" here is an attempt to classify a plane as being
/// in front or in the back of another one.
#[derive(Debug)]
pub enum PlaneCut<T> {
  /// The planes are one the same geometrical plane.
  Sibling(T),
  /// Planes are different, thus we can either determine that
  /// our plane is completely in front/back of another one,
  /// or split it into these sub-groups.
  Cut {
    /// Sub-planes in front of the base plane.
    front: Vec<T>,
    /// Sub-planes in the back of the base plane.
    back: Vec<T>,
  },
}

use crate::*;

impl<T> AbstractTree for BspNode<T> {
  fn visit_children(&self, mut visitor: impl FnMut(&Self)) {
    if let Some(n) = self.front.as_ref() {
      visitor(n.as_ref())
    }
    if let Some(n) = self.back.as_ref() {
      visitor(n.as_ref())
    }
  }
}

/// A plane abstracted to the matter of partitioning.
pub trait Plane: Sized + Clone {
  /// Try to cut a different plane by this one.
  fn cut(&self, plane: Self) -> PlaneCut<Self>;
  /// Check if a different plane is aligned in the same direction
  /// as this one.
  fn is_aligned(&self, plane: &Self) -> bool;
}

/// Add a list of planes to a particular front/end branch of some root node.
fn add_side<T: Plane>(side: &mut Option<Box<BspNode<T>>>, mut planes: Vec<T>) {
  if !planes.is_empty() {
    if side.is_none() {
      *side = Some(Box::new(BspNode::new()));
    }
    let node = side.as_mut().unwrap();
    for p in planes.drain(..) {
      node.insert(p)
    }
  }
}

/// A node in the `BspTree`, which can be considered a tree itself.
#[derive(Clone, Debug)]
pub struct BspNode<T> {
  values: Vec<T>,
  front: Option<Box<BspNode<T>>>,
  back: Option<Box<BspNode<T>>>,
}

impl<T> BspNode<T> {
  /// Create a new node.
  pub fn new() -> Self {
    BspNode {
      values: Vec::new(),
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

impl<T: Plane> BspNode<T> {
  /// Insert a value into the sub-tree starting with this node.
  /// This operation may spawn additional leafs/branches of the tree.
  pub fn insert(&mut self, value: T) {
    if self.values.is_empty() {
      self.values.push(value);
      return;
    }
    match self.values[0].cut(value) {
      PlaneCut::Sibling(value) => self.values.push(value),
      PlaneCut::Cut { front, back } => {
        add_side(&mut self.front, front);
        add_side(&mut self.back, back);
      }
    }
  }

  /// Build the draw order of this sub-tree into an `out` vector,
  /// so that the contained planes are sorted back to front according
  /// to the view vector defined as the `base` plane front direction.
  pub fn order(&self, base: &T, out: &mut Vec<T>) {
    let (former, latter) = match self.values.first() {
      None => return,
      Some(first) if base.is_aligned(first) => (self.front.as_ref(), self.back.as_ref()),
      Some(_) => (self.back.as_ref(), self.front.as_ref()),
    };

    if let Some(node) = former {
      node.order(base, out);
    }

    out.extend_from_slice(&self.values);

    if let Some(node) = latter {
      node.order(base, out);
    }
  }
}
