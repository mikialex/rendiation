use crate::*;

impl Node<f32> {
  pub fn near_than(&self, depth: Node<f32>, rev: bool) -> Node<bool> {
    if rev {
      self.greater_than(depth)
    } else {
      self.less_than(depth)
    }
  }

  pub fn near_equal_than(&self, depth: Node<f32>, rev: bool) -> Node<bool> {
    if rev {
      self.greater_equal_than(depth)
    } else {
      self.less_equal_than(depth)
    }
  }

  pub fn further_than(&self, depth: Node<f32>, rev: bool) -> Node<bool> {
    if rev {
      self.less_than(depth)
    } else {
      self.greater_than(depth)
    }
  }

  pub fn further_equal_than(&self, depth: Node<f32>, rev: bool) -> Node<bool> {
    if rev {
      self.less_equal_than(depth)
    } else {
      self.greater_equal_than(depth)
    }
  }
}
