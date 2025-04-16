use crate::*;

pub trait IntoShaderIterator {
  type ShaderIter: ShaderIterator;
  fn into_shader_iter(self) -> Self::ShaderIter;
}

pub type ItemOfIntoShaderIter<T> = <<T as IntoShaderIterator>::ShaderIter as ShaderIterator>::Item;

pub trait IntoShaderIteratorExt: IntoShaderIterator + Sized {
  fn map<F>(self, map: F) -> ShaderIntoIterMap<Self, F> {
    ShaderIntoIterMap {
      internal: self,
      map,
    }
  }
  fn zip<U>(self, other: U) -> ShaderIntoIterZip<Self, U> {
    ShaderIntoIterZip { a: self, b: other }
  }
}
impl<T: IntoShaderIterator> IntoShaderIteratorExt for T {}

#[derive(Clone)]
pub struct ShaderIntoIterMap<T, F> {
  internal: T,
  map: F,
}

impl<T, F, U> IntoShaderIterator for ShaderIntoIterMap<T, F>
where
  T: IntoShaderIterator,
  F: Fn(ItemOfIntoShaderIter<T>) -> U + 'static,
{
  type ShaderIter = impl ShaderIterator<Item = U>;
  fn into_shader_iter(self) -> Self::ShaderIter {
    self.internal.into_shader_iter().map(self.map)
  }
}

#[derive(Clone)]
pub struct ShaderIntoIterZip<T, U> {
  a: T,
  b: U,
}

impl<T, U> IntoShaderIterator for ShaderIntoIterZip<T, U>
where
  T: IntoShaderIterator,
  U: IntoShaderIterator,
{
  type ShaderIter = impl ShaderIterator<Item = (ItemOfIntoShaderIter<T>, ItemOfIntoShaderIter<U>)>;
  fn into_shader_iter(self) -> Self::ShaderIter {
    self.a.into_shader_iter().zip(self.b.into_shader_iter())
  }
}

impl IntoShaderIterator for u32 {
  type ShaderIter = StepTo;

  fn into_shader_iter(self) -> Self::ShaderIter {
    StepTo::new(val(self))
  }
}

impl IntoShaderIterator for Node<u32> {
  type ShaderIter = StepTo;

  fn into_shader_iter(self) -> Self::ShaderIter {
    StepTo::new(self)
  }
}

impl IntoShaderIterator for Node<Vec2<u32>> {
  type ShaderIter = ForRange;

  fn into_shader_iter(self) -> Self::ShaderIter {
    ForRange::ranged(self)
  }
}

impl<AT, T: ShaderSizedValueNodeType> IntoShaderIterator for StaticLengthArrayView<AT, T> {
  type ShaderIter = ShaderStaticArrayIter<AT, T>;

  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderStaticArrayIter {
      cursor: val(0_u32).make_local_var(),
      len: self.len,
      array: self,
    }
  }
}

impl<T: ShaderSizedValueNodeType> IntoShaderIterator for DynLengthArrayView<T> {
  type ShaderIter = ShaderDynArrayIter<T>;

  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderDynArrayIter {
      cursor: val(0_u32).make_local_var(),
      len: self.array_length(),
      array: self,
    }
  }
}

impl<AT, T: ShaderSizedValueNodeType> IntoShaderIterator for StaticLengthArrayReadonlyView<AT, T> {
  type ShaderIter = ShaderStaticArrayReadonlyIter<AT, T>;

  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderStaticArrayReadonlyIter {
      cursor: val(0_u32).make_local_var(),
      len: self.len,
      array: self,
    }
  }
}

impl<T: ShaderSizedValueNodeType> IntoShaderIterator for DynLengthArrayReadonlyView<T> {
  type ShaderIter = ShaderDynArrayReadonlyIter<T>;

  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderDynArrayReadonlyIter {
      cursor: val(0_u32).make_local_var(),
      len: self.array_length(),
      array: self,
    }
  }
}
