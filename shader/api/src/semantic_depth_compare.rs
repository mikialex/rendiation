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

  /// Evaluate the fixed function depth test in shader, self is the incoming fragment depth, and
  /// the stored is the depth in depth buffer. Return true if the test passes.
  pub fn depth_test_by(
    &self,
    compare: wgpu_types::CompareFunction,
    stored: Node<f32>,
  ) -> Node<bool> {
    use wgpu_types::CompareFunction::*;
    match compare {
      Never => val(false),
      Less => self.less_than(stored),
      Equal => self.equals(stored),
      LessEqual => self.less_equal_than(stored),
      Greater => self.greater_than(stored),
      NotEqual => self.not_equals(stored),
      GreaterEqual => self.greater_equal_than(stored),
      Always => val(true),
    }
  }
}
