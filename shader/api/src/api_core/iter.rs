use crate::*;

pub trait ShaderIterator {
  type Item;
  // we do not have sum type(enum) in shader, so we have to return extra flag to indicate if the
  // value is valid.
  fn shader_next(&self) -> (Node<bool>, Self::Item);
}

pub trait IntoShaderIterator {
  type ShaderIter: ShaderIterator;
  fn into_shader_iter(self) -> Self::ShaderIter;
}

pub trait ShaderIteratorExt: ShaderIterator + Sized {
  fn for_each(self, visitor: impl Fn(Self::Item, &LoopCtx)) {
    loop_by(|cx| {
      let (has_next, next) = self.shader_next();
      if_by(has_next.not(), || {
        cx.do_break();
      });
      visitor(next, &cx);
    });
  }

  fn map<F>(self, f: F) -> ShaderMapIter<Self, F> {
    ShaderMapIter { iter: self, f }
  }

  fn filter<F>(self, f: F) -> ShaderFilterIter<Self, F> {
    ShaderFilterIter { iter: self, f }
  }

  fn enumerate(self) -> ShaderEnumeratorIter<Self> {
    ShaderEnumeratorIter {
      iter: self,
      counter: val(0).make_local_var(),
    }
  }

  fn take_while<F>(self, f: F) -> ShaderTakeWhileIter<Self, F> {
    ShaderTakeWhileIter { iter: self, f }
  }
}
impl<T: ShaderIterator + Sized> ShaderIteratorExt for T {}

impl ShaderIterator for LocalVarNode<u32> {
  type Item = Node<u32>;

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let current = self.load();
    self.store(current - val(1));
    (current.equals(val(0)).not(), current)
  }
}

pub struct UniformArrayIter<T, const U: usize> {
  cursor: LocalVarNode<u32>,
  array: UniformNode<Shader140Array<T, U>>,
}

impl<T: ShaderNodeType, const U: usize> ShaderIterator for UniformArrayIter<T, U> {
  type Item = (Node<u32>, UniformNode<T>);

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let current = self.cursor.load();
    let next = current + val(1);
    self.cursor.store(next);
    let has_next = current.less_than(val(U as u32));

    // should we do the clamp by ourself?
    let uniform = self.array.index(next.min(val(U as u32)));
    (has_next, (next, uniform))
  }
}

impl<T: ShaderNodeType, const U: usize> IntoShaderIterator for UniformNode<Shader140Array<T, U>> {
  type ShaderIter = UniformArrayIter<T, U>;

  fn into_shader_iter(self) -> Self::ShaderIter {
    UniformArrayIter {
      cursor: val(0).make_local_var(),
      array: self,
    }
  }
}

pub struct ShaderFilterIter<T, F> {
  iter: T,
  f: F,
}

impl<T, F, TT> ShaderIterator for ShaderFilterIter<T, F>
where
  T: ShaderIterator<Item = Node<TT>>,
  TT: ShaderSizedValueNodeType + Default,
  F: Fn(&T::Item) -> Node<bool>,
{
  type Item = T::Item;

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let has_next = val(false).make_local_var();
    let item = zeroed_val().make_local_var();
    loop_by(|cx| {
      let (inner_has_next, inner) = self.iter.shader_next();
      if_by(inner_has_next.not(), || {
        cx.do_break();
      });
      if_by((self.f)(&inner), || {
        has_next.store(val(true));
        item.store_unchecked(inner);
      });
    });
    (has_next.load(), item.load_unchecked())
  }
}

pub struct ShaderMapIter<T, F> {
  iter: T,
  f: F,
}

impl<T, F, TT, MTT> ShaderIterator for ShaderMapIter<T, F>
where
  T: ShaderIterator<Item = Node<TT>>,
  TT: ShaderSizedValueNodeType,
  MTT: ShaderSizedValueNodeType,
  F: Fn(Node<TT>) -> Node<MTT>,
{
  type Item = Node<MTT>;

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let (inner_has_next, inner) = self.iter.shader_next();
    let item = zeroed_val().make_local_var();
    if_by(inner_has_next, || {
      item.store_unchecked((self.f)(inner));
    });
    (inner_has_next, item.load_unchecked())
  }
}

pub struct ShaderEnumeratorIter<T> {
  iter: T,
  counter: LocalVarNode<u32>,
}

impl<T: ShaderIterator> ShaderIterator for ShaderEnumeratorIter<T> {
  type Item = (Node<u32>, T::Item);

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let next = self.counter.load();
    self.counter.store(next + val(1));
    let (inner_has_next, inner_next) = self.iter.shader_next();
    (inner_has_next, (next, inner_next))
  }
}

pub struct ShaderTakeWhileIter<T, F> {
  iter: T,
  f: F,
}

impl<T, F> ShaderIterator for ShaderTakeWhileIter<T, F>
where
  T: ShaderIterator,
  F: Fn(&T::Item) -> Node<bool>,
{
  type Item = T::Item;

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let (inner_has_next, inner) = self.iter.shader_next();
    (inner_has_next.and((self.f)(&inner)), inner)
  }
}
