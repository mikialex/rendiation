use crate::*;

pub trait IntoShaderIterator {
  type ShaderIter: ShaderIterator;
  fn into_shader_iter(self) -> Self::ShaderIter;
}

pub trait IntoShaderIteratorExt: IntoShaderIterator + Sized {
  fn map<F>(self, map: F) -> ShaderIntoIterMap<Self, F> {
    ShaderIntoIterMap {
      internal: self,
      map,
    }
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
  F: Fn(<T::ShaderIter as ShaderIterator>::Item) -> U + 'static,
{
  type ShaderIter = impl ShaderIterator<Item = U>;

  fn into_shader_iter(self) -> Self::ShaderIter {
    self.internal.into_shader_iter().map(self.map)
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
