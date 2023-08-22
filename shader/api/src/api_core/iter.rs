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

  fn sum<T>(self) -> Self::Item
  where
    Self: ShaderIterator<Item = Node<T>>,
    T: ShaderSizedValueNodeType,
    Node<T>: Add<Output = Node<T>>,
  {
    let value = zeroed_val::<T>().make_local_var();
    self.for_each(|item, _| value.store(value.load() + item));
    value.load()
  }

  fn map<F: Fn(I) -> O, I, O>(self, f: F) -> ShaderMapIter<Self, F> {
    ShaderMapIter { iter: self, f }
  }

  fn filter<F: Fn(&I) -> Node<bool>, I>(self, f: F) -> ShaderFilterIter<Self, F> {
    ShaderFilterIter { iter: self, f }
  }

  fn enumerate(self) -> ShaderEnumeratorIter<Self> {
    ShaderEnumeratorIter {
      iter: self,
      counter: val(0).make_local_var(),
    }
  }

  fn take_while<F: Fn(&I) -> Node<bool>, I>(self, f: F) -> ShaderTakeWhileIter<Self, F> {
    ShaderTakeWhileIter { iter: self, f }
  }

  fn clamp_by<T>(self, count: Node<u32>) -> impl ShaderIterator<Item = (Node<u32>, Node<T>)>
  where
    Self: ShaderIterator<Item = (Node<u32>, Node<T>)>,
  {
    self.take_while(move |&(idx, _): &(Node<u32>, Node<T>)| idx.less_than(count))
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
    let current_next = self.cursor.load();
    self.cursor.store(current_next + val(1));
    let has_next = current_next.less_than(val(U as u32));

    // should we do the clamp by ourself?
    assert!(U >= 1);
    let uniform = self.array.index(current_next.min(val(U as u32 - 1)));
    (has_next, (current_next, uniform))
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
  TT: ShaderSizedValueNodeType,
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
        item.store(inner);
      });
    });
    (has_next.load(), item.load())
  }
}

pub struct ShaderMapIter<T, F> {
  iter: T,
  f: F,
}

impl<T, F, TT> ShaderIterator for ShaderMapIter<T, F>
where
  T: ShaderIterator,
  T::Item: Copy,
  TT: ShaderSizedValueNodeType,
  F: Fn(T::Item) -> Node<TT>,
{
  type Item = Node<TT>;

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let (inner_has_next, inner) = self.iter.shader_next();
    let item = zeroed_val().make_local_var();
    if_by(inner_has_next, || {
      item.store((self.f)(inner));
    });
    (inner_has_next, item.load())
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
