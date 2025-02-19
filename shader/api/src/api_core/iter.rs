use crate::*;

pub trait ShaderIterator {
  type Item;
  // we do not have sum type(enum) in shader, so we have to return extra flag to indicate if the
  // value is valid.
  fn shader_next(&self) -> (Node<bool>, Self::Item);
}

impl<T> ShaderIterator for Box<dyn ShaderIterator<Item = T>> {
  type Item = T;

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    (**self).shader_next()
  }
}

pub trait IntoShaderIterator {
  type ShaderIter: ShaderIterator;
  fn into_shader_iter(self) -> Self::ShaderIter;
}

pub trait ShaderIteratorExt: ShaderIterator + Sized {
  fn for_each(self, visitor: impl FnOnce(Self::Item, &LoopCtx)) {
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

  fn zip<T>(self, other: T) -> ShaderZipIter<Self, T> {
    ShaderZipIter {
      iter1: self,
      iter2: other,
    }
  }

  fn filter_map<F: Fn(I) -> (Node<bool>, Node<O>), I, O>(
    self,
    f: F,
  ) -> ShaderFilterMapIter<Self, F> {
    ShaderFilterMapIter { iter: self, f }
  }

  fn enumerate(self) -> ShaderEnumeratorIter<Self> {
    ShaderEnumeratorIter {
      iter: self,
      counter: val(0_u32).make_local_var(),
    }
  }

  fn take_while<F: Fn(&I) -> Node<bool>, I>(self, f: F) -> ShaderTakeWhileIter<Self, F> {
    ShaderTakeWhileIter { iter: self, f }
  }

  fn clamp_by<T>(self, count: Node<u32>) -> impl ShaderIterator<Item = (Node<u32>, T)>
  where
    Self: ShaderIterator<Item = (Node<u32>, T)>,
  {
    self.take_while(move |&(idx, _): &(Node<u32>, T)| idx.less_than(count))
  }

  fn flat_map<TT, F, I>(self, f: F) -> ShaderFlatMapIter<Self, I, TT, F>
  where
    F: Fn(Self::Item) -> I,
    I: ShaderAbstractRightValue,
    I::AbstractLeftValue: ShaderIterator<Item = Node<TT>>,
  {
    ShaderFlatMapIter {
      outer: self,
      inner: I::create_left_value_from_builder(&mut LocalLeftValueBuilder),
      f,
    }
  }
}
impl<T: ShaderIterator + Sized> ShaderIteratorExt for T {}

pub struct StepTo {
  to: Node<u32>,
  current: ShaderAccessorOf<u32>,
}

impl StepTo {
  fn new(to: Node<u32>) -> Self {
    Self {
      to,
      current: val(0_u32).make_local_var(),
    }
  }
}

impl ShaderIterator for StepTo {
  type Item = Node<u32>;

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let current = self.current.load();
    self.current.store(current + val(1));
    (current.equals(self.to).not(), current)
  }
}

pub struct ForRange {
  to: BoxedShaderLoadStore<Node<u32>>,
  current: BoxedShaderLoadStore<Node<u32>>,
}
impl ForRange {
  pub fn ranged(range: Node<Vec2<u32>>, builder: &mut impl LeftValueBuilder) -> Self {
    Self {
      to: builder.create_left_value(range.y()),
      current: builder.create_left_value(range.x()),
    }
  }
}

impl ShaderIterator for ForRange {
  type Item = Node<u32>;
  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let current = self.current.abstract_load();
    self.current.abstract_store(current + val(1));
    (current.equals(self.to.abstract_load()).not(), current)
  }
}

#[derive(Clone)]
pub struct ForRangeState {
  pub to: Node<u32>,
  pub current: Node<u32>,
}

impl ForRangeState {
  pub fn from_range(from_range: Node<Vec2<u32>>) -> Self {
    Self {
      current: from_range.x(),
      to: from_range.y(),
    }
  }
}
impl ShaderAbstractRightValue for ForRangeState {
  type AbstractLeftValue = ForRange;

  fn create_left_value_from_builder<B: LeftValueBuilder>(
    builder: &mut B,
  ) -> Self::AbstractLeftValue {
    ForRange {
      to: builder.create_left_value(val(0_u32)),
      current: builder.create_left_value(val(0)),
    }
  }
}
impl ShaderAbstractLeftValue for ForRange {
  type RightValue = ForRangeState;

  fn abstract_load(&self) -> Self::RightValue {
    ForRangeState {
      to: self.to.abstract_load(),
      current: self.current.abstract_load(),
    }
  }

  fn abstract_store(&self, payload: Self::RightValue) {
    self.to.abstract_store(payload.to);
    self.current.abstract_store(payload.current);
  }
}

#[derive(Clone)]
pub struct ShaderStaticArrayIter<AT, T> {
  cursor: ShaderAccessorOf<u32>,
  array: StaticLengthArrayAccessor<AT, T>,
  len: u32,
}

impl<AT, T: ShaderSizedValueNodeType> ShaderIterator for ShaderStaticArrayIter<AT, T> {
  type Item = (Node<u32>, ShaderAccessorOf<T>);

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let current_next = self.cursor.load();
    self.cursor.store(current_next + val(1));
    let has_next = current_next.less_than(val(self.len));

    // should we do the clamp by ourself?
    assert!(self.len >= 1);
    let uniform = self.array.index(current_next.min(val(self.len - 1)));
    (has_next, (current_next, uniform))
  }
}

#[derive(Clone)]
pub struct ShaderStaticArrayReadonlyIter<AT, T> {
  cursor: ShaderAccessorOf<u32>,
  array: StaticLengthArrayReadonlyAccessor<AT, T>,
  len: u32,
}

impl<AT, T: ShaderSizedValueNodeType> ShaderIterator for ShaderStaticArrayReadonlyIter<AT, T> {
  type Item = (Node<u32>, ShaderReadonlyAccessorOf<T>);

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let current_next = self.cursor.load();
    self.cursor.store(current_next + val(1));
    let has_next = current_next.less_than(val(self.len));

    // should we do the clamp by ourself?
    assert!(self.len >= 1);
    let uniform = self.array.index(current_next.min(val(self.len - 1)));
    (has_next, (current_next, uniform))
  }
}

#[derive(Clone)]
pub struct ShaderDynArrayIter<T> {
  cursor: ShaderAccessorOf<u32>,
  array: DynLengthArrayAccessor<T>,
  len: Node<u32>,
}

impl<T: ShaderSizedValueNodeType> ShaderIterator for ShaderDynArrayIter<T> {
  type Item = (Node<u32>, ShaderAccessorOf<T>);

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let current_next = self.cursor.load();
    self.cursor.store(current_next + val(1));
    let has_next = current_next.less_than(self.len);
    let data = self.array.index(current_next.min(self.len - val(1)));
    (has_next, (current_next, data))
  }
}

#[derive(Clone)]
pub struct ShaderDynArrayReadonlyIter<T> {
  cursor: ShaderAccessorOf<u32>,
  array: DynLengthArrayReadonlyAccessor<T>,
  len: Node<u32>,
}

impl<T: ShaderSizedValueNodeType> ShaderIterator for ShaderDynArrayReadonlyIter<T> {
  type Item = (Node<u32>, ShaderReadonlyAccessorOf<T>);

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let current_next = self.cursor.load();
    self.cursor.store(current_next + val(1));
    let has_next = current_next.less_than(self.len);
    let data = self.array.index(current_next.min(self.len - val(1)));
    (has_next, (current_next, data))
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

impl<AT, T: ShaderSizedValueNodeType> IntoShaderIterator for StaticLengthArrayAccessor<AT, T> {
  type ShaderIter = ShaderStaticArrayIter<AT, T>;

  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderStaticArrayIter {
      cursor: val(0_u32).make_local_var(),
      len: self.len,
      array: self,
    }
  }
}

impl<T: ShaderSizedValueNodeType> IntoShaderIterator for DynLengthArrayAccessor<T> {
  type ShaderIter = ShaderDynArrayIter<T>;

  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderDynArrayIter {
      cursor: val(0_u32).make_local_var(),
      len: self.array_length(),
      array: self,
    }
  }
}

impl<AT, T: ShaderSizedValueNodeType> IntoShaderIterator
  for StaticLengthArrayReadonlyAccessor<AT, T>
{
  type ShaderIter = ShaderStaticArrayReadonlyIter<AT, T>;

  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderStaticArrayReadonlyIter {
      cursor: val(0_u32).make_local_var(),
      len: self.len,
      array: self,
    }
  }
}

impl<T: ShaderSizedValueNodeType> IntoShaderIterator for DynLengthArrayReadonlyAccessor<T> {
  type ShaderIter = ShaderDynArrayReadonlyIter<T>;

  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderDynArrayReadonlyIter {
      cursor: val(0_u32).make_local_var(),
      len: self.array_length(),
      array: self,
    }
  }
}

#[derive(Clone)]
pub struct ShaderFilterIter<T, F> {
  iter: T,
  f: F,
}

impl<T, F, TT> ShaderIterator for ShaderFilterIter<T, F>
where
  T: ShaderIterator<Item = TT>,
  TT: ShaderAbstractRightValue + Default,
  F: Fn(TT) -> Node<bool>,
{
  type Item = T::Item;

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let has_next = val(false).make_local_var();
    let item = LocalLeftValueBuilder.create_left_value(TT::default());
    loop_by(|cx| {
      let (inner_has_next, inner) = self.iter.shader_next();
      if_by(inner_has_next.not(), || {
        cx.do_break();
      });
      if_by((self.f)(inner.clone()), || {
        has_next.store(val(true));
        item.abstract_store(inner);
      });
    });
    (has_next.load(), item.abstract_load())
  }
}

#[derive(Clone)]
pub struct ShaderFilterMapIter<T, F> {
  iter: T,
  f: F,
}

impl<T, F, TT, O> ShaderIterator for ShaderFilterMapIter<T, F>
where
  T: ShaderIterator<Item = Node<TT>>,
  TT: ShaderSizedValueNodeType,
  O: ShaderSizedValueNodeType,
  F: Fn(T::Item) -> (Node<bool>, Node<O>),
{
  type Item = Node<O>;

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let has_next = val(false).make_local_var();
    let item = zeroed_val::<O>().make_local_var();
    loop_by(|cx| {
      let (inner_has_next, inner) = self.iter.shader_next();
      if_by(inner_has_next.not(), || {
        cx.do_break();
      });
      let (n, mapped) = (self.f)(inner);
      if_by(n, || {
        has_next.store(val(true));
        item.store(mapped);
        cx.do_break();
      });
    });
    (has_next.load(), item.load())
  }
}

#[derive(Clone)]
pub struct ShaderMapIter<T, F> {
  iter: T,
  f: F,
}

impl<T, F, TT> ShaderIterator for ShaderMapIter<T, F>
where
  T: ShaderIterator,
  T::Item: Clone,
  TT: ShaderAbstractRightValue + Default,
  F: Fn(T::Item) -> TT,
{
  type Item = TT;

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let (inner_has_next, inner) = self.iter.shader_next();
    let item = LocalLeftValueBuilder.create_left_value(TT::default());
    if_by(inner_has_next, || {
      item.abstract_store((self.f)(inner));
    });
    (inner_has_next, item.abstract_load())
  }
}

#[derive(Clone)]
pub struct ShaderZipIter<T1, T2> {
  iter1: T1,
  iter2: T2,
}

impl<T1, T2> ShaderIterator for ShaderZipIter<T1, T2>
where
  T1: ShaderIterator,
  T2: ShaderIterator,
{
  type Item = (T1::Item, T2::Item);

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let (inner1_has_next, inner1) = self.iter1.shader_next();
    let (inner2_has_next, inner2) = self.iter2.shader_next();

    let has_next = inner1_has_next.and(inner2_has_next);
    let next = (inner1, inner2);

    (has_next, next)
  }
}

pub struct ShaderEnumeratorIter<T> {
  iter: T,
  counter: ShaderAccessorOf<u32>,
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

#[derive(Clone)]
pub struct ShaderFlatMapIter<Outer, Inner, IItem, F>
where
  Outer: ShaderIterator,
  Inner: ShaderAbstractRightValue,
  Inner::AbstractLeftValue: ShaderIterator<Item = Node<IItem>>,
  F: Fn(Outer::Item) -> Inner,
{
  outer: Outer,
  // initial inner must return has_next=false, Inner::create_left_value_from_builder(&mut LocalLeftValueBuilder);
  inner: Inner::AbstractLeftValue,
  // create inner from outer item, overwrite inner
  f: F,
}

impl<Outer, Inner, IItem, F> ShaderIterator for ShaderFlatMapIter<Outer, Inner, IItem, F>
where
  Outer: ShaderIterator,
  Inner: ShaderAbstractRightValue,
  Inner::AbstractLeftValue: ShaderIterator<Item = Node<IItem>>,
  IItem: ShaderSizedValueNodeType,
  F: Fn(Outer::Item) -> Inner,
{
  type Item = <Inner::AbstractLeftValue as ShaderIterator>::Item;

  fn shader_next(&self) -> (Node<bool>, Self::Item) {
    let has_next = val(false).make_local_var();
    let next = zeroed_val::<IItem>().make_local_var();

    // poll inner first
    let (inner_has_next, inner_next) = self.inner.shader_next();
    if_by(inner_has_next, || {
      has_next.store(val(true));
      next.store(inner_next);
    })
    .else_by(|| {
      // then poll outer to update inner

      let (outer_has_next, outer_next) = self.outer.shader_next();
      if_by(outer_has_next, || {
        let inner = (self.f)(outer_next); // inner updated
        self.inner.abstract_store(inner);

        // todo avoid code duplication?
        let (inner_has_next, inner_next) = self.inner.shader_next();
        if_by(inner_has_next, || {
          has_next.store(val(true));
          next.store(inner_next);
        });
      });
    });

    (has_next.load(), next.load())
  }
}
