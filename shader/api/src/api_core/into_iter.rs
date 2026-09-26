use std::ops::Range;

use crate::*;

/// The iterable, which creates a new [ShaderIterator] (with the new iteration state) each time.
pub trait IntoShaderIterator {
  type Item;
  type ShaderIter: ShaderIterator<Item = Self::Item>;
  fn into_shader_iter(self) -> Self::ShaderIter;
}

impl<T: ShaderIterator> IntoShaderIterator for T {
  type Item = T::Item;
  type ShaderIter = T;
  fn into_shader_iter(self) -> Self::ShaderIter {
    self
  }
}

/// 0..n
impl IntoShaderIterator for u32 {
  type Item = Node<u32>;
  type ShaderIter = ShaderRangeIter;
  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderRangeIter::new(ShaderRange::new(val(0), val(self)))
  }
}

/// 0..n
impl IntoShaderIterator for Node<u32> {
  type Item = Node<u32>;
  type ShaderIter = ShaderRangeIter;
  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderRangeIter::new(ShaderRange::new(val(0), self))
  }
}

/// (start, end) range, prefer the [Range] which is more explicit
impl IntoShaderIterator for Node<Vec2<u32>> {
  type Item = Node<u32>;
  type ShaderIter = ShaderRangeIter;
  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderRangeIter::new(ShaderRange::from_vec2(self))
  }
}

impl IntoShaderIterator for Range<Node<u32>> {
  type Item = Node<u32>;
  type ShaderIter = ShaderRangeIter;
  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderRangeIter::new(ShaderRange::new(self.start, self.end))
  }
}

impl IntoShaderIterator for Range<u32> {
  type Item = Node<u32>;
  type ShaderIter = ShaderRangeIter;
  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderRangeIter::new(ShaderRange::new(val(self.start), val(self.end)))
  }
}

impl<AT, T: ShaderSizedValueNodeType> IntoShaderIterator for StaticLengthArrayView<AT, T> {
  type Item = (Node<u32>, ShaderPtrOf<T>);
  type ShaderIter = ShaderIndexIter<Self>;
  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderIndexIter::new(self)
  }
}

impl<AT, T: ShaderSizedValueNodeType> IntoShaderIterator for StaticLengthArrayReadonlyView<AT, T> {
  type Item = (Node<u32>, ShaderReadonlyPtrOf<T>);
  type ShaderIter = ShaderIndexIter<Self>;
  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderIndexIter::new(self)
  }
}

impl<T: ShaderSizedValueNodeType> IntoShaderIterator for DynLengthArrayView<T> {
  type Item = (Node<u32>, ShaderPtrOf<T>);
  type ShaderIter = ShaderIndexIter<Self>;
  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderIndexIter::new(self)
  }
}

impl<T: ShaderSizedValueNodeType> IntoShaderIterator for DynLengthArrayReadonlyView<T> {
  type Item = (Node<u32>, ShaderReadonlyPtrOf<T>);
  type ShaderIter = ShaderIndexIter<Self>;
  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderIndexIter::new(self)
  }
}
