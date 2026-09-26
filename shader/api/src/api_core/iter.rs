use crate::*;

/// The result of polling a [ShaderIterator].
///
/// We do not have sum type(enum) in shader, so the flag indicates if the item is valid. The item is
/// constructed lazily in the region that the flag is true, so the item construction and the logic
/// of the adaptors (like the map closure) never run for the poll that ends the iteration.
pub struct ShaderIterNext<'a, T> {
  pub has_next: Node<bool>,
  item: Box<dyn FnOnce() -> T + 'a>,
}

impl<'a, T> ShaderIterNext<'a, T> {
  pub fn new(has_next: Node<bool>, item: impl FnOnce() -> T + 'a) -> Self {
    Self {
      has_next,
      item: Box::new(item),
    }
  }

  /// Construct the item. It must be called in the region that the `has_next` is true, and in the
  /// block where the iterator is polled or its child blocks, because the item may reference the
  /// expressions created by the poll.
  pub fn item(self) -> T {
    (self.item)()
  }
}

impl<'a, T: 'a> ShaderIterNext<'a, T> {
  /// Map the lazy item, the `f` is called when the item is constructed.
  pub fn map<U>(self, f: impl FnOnce(T) -> U + 'a) -> ShaderIterNext<'a, U> {
    ShaderIterNext::new(self.has_next, move || f(self.item()))
  }
}

impl<'a, T> ShaderIterNext<'a, T>
where
  T: ShaderAbstractRightValue,
  T::AbstractLeftValue: 'a,
{
  /// Search the next item in a loop. The `body` is the loop body, it calls the `found` with the
  /// item to take it and exit the loop, or exits the loop by the loop ctx when nothing is found.
  ///
  /// The found item is carried out of the loop by a local variable, so it must be a right value.
  pub fn search(body: impl FnOnce(&LoopCtx, &dyn Fn(T))) -> Self {
    let has_next = val(false).make_local_var();
    let item = T::create_left_value_from_builder(&mut LocalLeftValueBuilder);
    loop_by(|cx| {
      body(&cx, &|value: T| {
        has_next.store(val(true));
        item.abstract_store(value);
        cx.do_break();
      });
    });
    Self::new(has_next.load(), move || item.abstract_load())
  }
}

/// The iterator state is stored in shader variables and initialized where the iterator is
/// created, so the iterator should be created right where the iteration starts. To pass the
/// iterable around or iterate it multiple times, use the [IntoShaderIterator].
pub trait ShaderIterator {
  type Item;
  /// Advance the iterator state and return the lazy item, see [ShaderIterNext].
  ///
  /// Once the flag is false the iteration ends, and the iterator should not be polled again. The
  /// provided sources keep returning false after they are exhausted (fused), so the misuse does
  /// not loop forever.
  fn shader_next(&self) -> ShaderIterNext<'_, Self::Item>;

  /// Check the iterator is iterated in the scope where its state is created, called by the
  /// consumers before the iteration starts. The iterator that has state checks the
  /// [ShaderIterScope] captured when the state is created, the adaptor forwards it to the inner
  /// iterators.
  fn check_iter_scope(&self);
}

impl<'a, T> ShaderIterator for Box<dyn ShaderIterator<Item = T> + 'a> {
  type Item = T;

  fn shader_next(&self) -> ShaderIterNext<'_, Self::Item> {
    (**self).shader_next()
  }

  fn check_iter_scope(&self) {
    (**self).check_iter_scope()
  }
}

pub trait ShaderIteratorExt: ShaderIterator + Sized {
  fn for_each(self, visitor: impl FnOnce(Self::Item, &LoopCtx)) {
    self.check_iter_scope();
    loop_by(|cx| {
      let next = self.shader_next();
      if_by(next.has_next.not(), || {
        cx.do_break();
      });
      visitor(next.item(), &cx);
    });
  }

  /// accumulate the items into the state, the state must be a right value
  fn fold<S: ShaderAbstractRightValue>(self, init: S, f: impl FnOnce(S, Self::Item) -> S) -> S {
    let state = LocalLeftValueBuilder.create_left_value(init);
    self.for_each(|item, _| state.abstract_store(f(state.abstract_load(), item)));
    state.abstract_load()
  }

  fn sum<T>(self) -> Node<T>
  where
    Self: ShaderIterator<Item = Node<T>>,
    T: ShaderSizedValueNodeType,
    Node<T>: Add<Output = Node<T>>,
  {
    self.fold(zeroed_val(), |sum, item| sum + item)
  }

  /// count the items, the items are not constructed, so the lazy logic like the map closure does
  /// not run
  fn count(self) -> Node<u32> {
    self.check_iter_scope();
    let count = val(0_u32).make_local_var();
    loop_by(|cx| {
      let next = self.shader_next();
      if_by(next.has_next.not(), || {
        cx.do_break();
      });
      count.store(count.load() + val(1));
    });
    count.load()
  }

  /// if any item matches the predicate, the iteration stops at the first matched item
  fn any(self, f: impl FnOnce(Self::Item) -> Node<bool>) -> Node<bool> {
    let result = val(false).make_local_var();
    self.for_each(|item, cx| {
      if_by(f(item), || {
        result.store(val(true));
        cx.do_break();
      });
    });
    result.load()
  }

  /// if all items match the predicate, the iteration stops at the first unmatched item
  fn all(self, f: impl FnOnce(Self::Item) -> Node<bool>) -> Node<bool> {
    self.any(|item| f(item).not()).not()
  }

  /// the first item that matches the predicate, the item must be a right value
  fn find(self, f: impl FnOnce(&Self::Item) -> Node<bool>) -> ShaderOption<Self::Item>
  where
    Self::Item: ShaderAbstractRightValue,
  {
    self.check_iter_scope();
    let next = ShaderIterNext::search(|cx, found| {
      let next = self.shader_next();
      if_by(next.has_next.not(), || {
        cx.do_break();
      });
      let item = next.item();
      if_by(f(&item), || found(item));
    });
    // loading the search result variable is always valid, the payload is only meaningful when
    // is_some is true
    ShaderOption::new(next.has_next, next.item())
  }

  /// the index of the first item that matches the predicate
  fn position(self, f: impl FnOnce(Self::Item) -> Node<bool>) -> ShaderOption<Node<u32>> {
    let iter = self.enumerate();
    iter.check_iter_scope();
    let next = ShaderIterNext::search(|cx, found| {
      let next = iter.shader_next();
      if_by(next.has_next.not(), || {
        cx.do_break();
      });
      let (index, item) = next.item();
      if_by(f(item), || found(index));
    });
    ShaderOption::new(next.has_next, next.item())
  }

  fn map<O, F: Fn(Self::Item) -> O>(self, f: F) -> ShaderMapIter<Self, F> {
    ShaderMapIter { iter: self, f }
  }

  /// the item must be a right value, because the matched item is stored and carried out of the
  /// internal search loop, the pointer item can not be filtered directly, map it to value first.
  fn filter<F: Fn(&Self::Item) -> Node<bool>>(self, f: F) -> ShaderFilterIter<Self, F> {
    ShaderFilterIter { iter: self, f }
  }

  fn zip<T: IntoShaderIterator>(self, other: T) -> ShaderZipIter<Self, T::ShaderIter> {
    ShaderZipIter {
      iter1: self,
      iter2: other.into_shader_iter(),
    }
  }

  /// the `f` returns if the item is kept and the mapped item, the mapped item must be a right value
  /// because it is carried out of the internal search loop, the input item has no requirement.
  fn filter_map<O, F: Fn(Self::Item) -> (Node<bool>, O)>(
    self,
    f: F,
  ) -> ShaderFilterMapIter<Self, F> {
    ShaderFilterMapIter { iter: self, f }
  }

  fn enumerate(self) -> ShaderEnumeratorIter<Self> {
    ShaderEnumeratorIter {
      iter: self,
      counter: val(0_u32).make_local_var(),
      scope: ShaderIterScope::capture(),
    }
  }

  /// the item must be a right value, because the predicate requires the item constructed, the
  /// item is stored and carried out of the region that the inner iterator has next.
  fn take_while<F: Fn(&Self::Item) -> Node<bool>>(self, f: F) -> ShaderTakeWhileIter<Self, F> {
    ShaderTakeWhileIter { iter: self, f }
  }

  /// take the first `count` items. When the count is reached, the inner iterator is still polled
  /// once but its item is not constructed.
  fn take(self, count: Node<u32>) -> ShaderTakeIter<Self> {
    ShaderTakeIter {
      iter: self,
      count,
      taken: val(0_u32).make_local_var(),
      scope: ShaderIterScope::capture(),
    }
  }

  /// limit the iteration by the item index, the same as `take(count)` because the item index of
  /// the sources (like the arrays) starts from 0.
  fn clamp_by<T>(self, count: Node<u32>) -> ShaderTakeIter<Self>
  where
    Self: ShaderIterator<Item = (Node<u32>, T)>,
  {
    self.take(count)
  }

  /// The `f` creates the inner iterator state ([ShaderIterState], like the [ShaderRange]) from
  /// the outer item, the state is stored into the inner iterator to reset it. The inner item must
  /// be a right value, because it is carried out of the internal search loop.
  fn flat_map<I, F>(self, f: F) -> ShaderFlatMapIter<Self, I, F>
  where
    F: Fn(Self::Item) -> I,
    I: ShaderIterState,
  {
    ShaderFlatMapIter {
      outer: self,
      inner: I::create_left_value_from_builder(&mut LocalLeftValueBuilder),
      f,
    }
  }
}
impl<T: ShaderIterator + Sized> ShaderIteratorExt for T {}

/// The iterator state as a right value, the iterator is its left value, so storing a new state
/// resets the iterator. The flat_map creates the inner iterator state from the outer item.
///
/// The left value created by the [LeftValueBuilder] must be an exhausted iterator.
pub trait ShaderIterState: ShaderAbstractRightValue<AbstractLeftValue: ShaderIterator> {}

/// The range from start (inclusive) to end (exclusive), empty if start >= end. Iterated by the
/// [ShaderRangeIter], and it is the [ShaderIterState] of it.
#[derive(Clone, Copy)]
pub struct ShaderRange {
  pub start: Node<u32>,
  pub end: Node<u32>,
}

impl ShaderRange {
  pub fn new(start: Node<u32>, end: Node<u32>) -> Self {
    Self { start, end }
  }
  /// the range is (start, end)
  pub fn from_vec2(range: Node<Vec2<u32>>) -> Self {
    Self::new(range.x(), range.y())
  }
}

impl ShaderAbstractRightValue for ShaderRange {
  type AbstractLeftValue = ShaderRangeIter;

  /// the created range is empty, as the [ShaderIterState] requires
  fn create_left_value_from_builder<B: LeftValueBuilder>(
    builder: &mut B,
  ) -> Self::AbstractLeftValue {
    ShaderRangeIter::new_from(ShaderRange::new(val(0), val(0)), builder)
  }
}
impl ShaderIterState for ShaderRange {}

impl IntoShaderIterator for ShaderRange {
  type Item = Node<u32>;
  type ShaderIter = ShaderRangeIter;
  fn into_shader_iter(self) -> Self::ShaderIter {
    ShaderRangeIter::new(self)
  }
}

/// Iterate the [ShaderRange].
pub struct ShaderRangeIter {
  current: BoxedShaderLoadStore<Node<u32>>,
  end: BoxedShaderLoadStore<Node<u32>>,
  scope: ShaderIterScope,
}

impl ShaderRangeIter {
  pub fn new(range: ShaderRange) -> Self {
    Self::new_from(range, &mut LocalLeftValueBuilder)
  }
  /// the iteration state is created by the builder
  pub fn new_from(range: ShaderRange, builder: &mut impl LeftValueBuilder) -> Self {
    Self {
      current: builder.create_left_value(range.start),
      end: builder.create_left_value(range.end),
      scope: ShaderIterScope::capture(),
    }
  }
}

impl ShaderIterator for ShaderRangeIter {
  type Item = Node<u32>;
  fn shader_next(&self) -> ShaderIterNext<'_, Self::Item> {
    let current = self.current.abstract_load();
    self.current.abstract_store(current + val(1));
    // use less than instead of not equal, so the empty range(start >= end) does not loop forever,
    // and the exhausted range keeps returning false
    let has_next = current.less_than(self.end.abstract_load());
    ShaderIterNext::new(has_next, move || current)
  }

  fn check_iter_scope(&self) {
    self.scope.check()
  }
}

impl ShaderAbstractLeftValue for ShaderRangeIter {
  type RightValue = ShaderRange;

  fn abstract_load(&self) -> Self::RightValue {
    ShaderRange::new(self.current.abstract_load(), self.end.abstract_load())
  }

  fn abstract_store(&self, payload: Self::RightValue) {
    self.current.abstract_store(payload.start);
    self.end.abstract_store(payload.end);
  }
}

/// The random accessible collection, iterated by the [ShaderIndexIter].
pub trait ShaderIndexable {
  type Item;
  fn shader_len(&self) -> Node<u32>;
  /// the index is in bounds when it is called by the iterator
  fn shader_index(&self, index: Node<u32>) -> Self::Item;
}

impl<AT, T: ShaderSizedValueNodeType> ShaderIndexable for StaticLengthArrayView<AT, T> {
  type Item = ShaderPtrOf<T>;
  fn shader_len(&self) -> Node<u32> {
    val(self.len)
  }
  fn shader_index(&self, index: Node<u32>) -> Self::Item {
    self.index(index)
  }
}

impl<AT, T: ShaderSizedValueNodeType> ShaderIndexable for StaticLengthArrayReadonlyView<AT, T> {
  type Item = ShaderReadonlyPtrOf<T>;
  fn shader_len(&self) -> Node<u32> {
    val(self.len)
  }
  fn shader_index(&self, index: Node<u32>) -> Self::Item {
    self.index(index)
  }
}

impl<T: ShaderSizedValueNodeType> ShaderIndexable for DynLengthArrayView<T> {
  type Item = ShaderPtrOf<T>;
  fn shader_len(&self) -> Node<u32> {
    self.array_length()
  }
  fn shader_index(&self, index: Node<u32>) -> Self::Item {
    self.index(index)
  }
}

impl<T: ShaderSizedValueNodeType> ShaderIndexable for DynLengthArrayReadonlyView<T> {
  type Item = ShaderReadonlyPtrOf<T>;
  fn shader_len(&self) -> Node<u32> {
    self.array_length()
  }
  fn shader_index(&self, index: Node<u32>) -> Self::Item {
    self.index(index)
  }
}

/// Iterate the [ShaderIndexable] collection, the item is `(index, item)`.
pub struct ShaderIndexIter<A> {
  cursor: ShaderPtrOf<u32>,
  len: Node<u32>,
  array: A,
  scope: ShaderIterScope,
}

impl<A: ShaderIndexable> ShaderIndexIter<A> {
  pub fn new(array: A) -> Self {
    Self {
      cursor: val(0_u32).make_local_var(),
      len: array.shader_len(),
      array,
      scope: ShaderIterScope::capture(),
    }
  }

  /// iterate the first `len` items, the `len` is clamped by the collection length
  pub fn with_len_clamp(array: A, len: Node<u32>) -> Self {
    Self {
      cursor: val(0_u32).make_local_var(),
      len: len.min(array.shader_len()),
      array,
      scope: ShaderIterScope::capture(),
    }
  }
}

impl<A: ShaderIndexable> ShaderIterator for ShaderIndexIter<A> {
  type Item = (Node<u32>, A::Item);

  fn shader_next(&self) -> ShaderIterNext<'_, Self::Item> {
    let index = self.cursor.load();
    self.cursor.store(index + val(1));
    ShaderIterNext::new(index.less_than(self.len), move || {
      (index, self.array.shader_index(index))
    })
  }

  fn check_iter_scope(&self) {
    self.scope.check()
  }
}

pub struct ShaderFilterIter<T, F> {
  iter: T,
  f: F,
}

impl<T, F> ShaderIterator for ShaderFilterIter<T, F>
where
  T: ShaderIterator,
  T::Item: ShaderAbstractRightValue,
  F: Fn(&T::Item) -> Node<bool>,
{
  type Item = T::Item;

  fn shader_next(&self) -> ShaderIterNext<'_, Self::Item> {
    ShaderIterNext::search(|cx, found| {
      let next = self.iter.shader_next();
      if_by(next.has_next.not(), || {
        cx.do_break();
      });
      let item = next.item();
      if_by((self.f)(&item), || found(item));
    })
  }

  fn check_iter_scope(&self) {
    self.iter.check_iter_scope()
  }
}

pub struct ShaderFilterMapIter<T, F> {
  iter: T,
  f: F,
}

impl<T, F, O> ShaderIterator for ShaderFilterMapIter<T, F>
where
  T: ShaderIterator,
  O: ShaderAbstractRightValue,
  F: Fn(T::Item) -> (Node<bool>, O),
{
  type Item = O;

  fn shader_next(&self) -> ShaderIterNext<'_, Self::Item> {
    ShaderIterNext::search(|cx, found| {
      let next = self.iter.shader_next();
      if_by(next.has_next.not(), || {
        cx.do_break();
      });
      let (keep, mapped) = (self.f)(next.item());
      if_by(keep, || found(mapped));
    })
  }

  fn check_iter_scope(&self) {
    self.iter.check_iter_scope()
  }
}

pub struct ShaderMapIter<T, F> {
  iter: T,
  f: F,
}

impl<T, F, TT> ShaderIterator for ShaderMapIter<T, F>
where
  T: ShaderIterator,
  F: Fn(T::Item) -> TT,
{
  type Item = TT;

  fn shader_next(&self) -> ShaderIterNext<'_, Self::Item> {
    self.iter.shader_next().map(|item| (self.f)(item))
  }

  fn check_iter_scope(&self) {
    self.iter.check_iter_scope()
  }
}

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

  fn shader_next(&self) -> ShaderIterNext<'_, Self::Item> {
    let next1 = self.iter1.shader_next();
    let next2 = self.iter2.shader_next();
    let has_next = next1.has_next.and(next2.has_next);
    ShaderIterNext::new(has_next, move || (next1.item(), next2.item()))
  }

  fn check_iter_scope(&self) {
    self.iter1.check_iter_scope();
    self.iter2.check_iter_scope();
  }
}

pub struct ShaderEnumeratorIter<T> {
  iter: T,
  counter: ShaderPtrOf<u32>,
  scope: ShaderIterScope,
}

impl<T: ShaderIterator> ShaderIterator for ShaderEnumeratorIter<T> {
  type Item = (Node<u32>, T::Item);

  fn shader_next(&self) -> ShaderIterNext<'_, Self::Item> {
    let index = self.counter.load();
    self.counter.store(index + val(1));
    self.iter.shader_next().map(move |item| (index, item))
  }

  fn check_iter_scope(&self) {
    self.scope.check();
    self.iter.check_iter_scope();
  }
}

pub struct ShaderTakeWhileIter<T, F> {
  iter: T,
  f: F,
}

impl<T, F> ShaderIterator for ShaderTakeWhileIter<T, F>
where
  T: ShaderIterator,
  T::Item: ShaderAbstractRightValue,
  F: Fn(&T::Item) -> Node<bool>,
{
  type Item = T::Item;

  fn shader_next(&self) -> ShaderIterNext<'_, Self::Item> {
    let has_next = val(false).make_local_var();
    let item = T::Item::create_left_value_from_builder(&mut LocalLeftValueBuilder);
    let next = self.iter.shader_next();
    if_by(next.has_next, || {
      let inner = next.item();
      if_by((self.f)(&inner), || {
        has_next.store(val(true));
        item.abstract_store(inner);
      });
    });
    ShaderIterNext::new(has_next.load(), move || item.abstract_load())
  }

  fn check_iter_scope(&self) {
    self.iter.check_iter_scope()
  }
}

pub struct ShaderTakeIter<T> {
  iter: T,
  count: Node<u32>,
  taken: ShaderPtrOf<u32>,
  scope: ShaderIterScope,
}

impl<T: ShaderIterator> ShaderIterator for ShaderTakeIter<T> {
  type Item = T::Item;

  fn shader_next(&self) -> ShaderIterNext<'_, Self::Item> {
    let taken = self.taken.load();
    self.taken.store(taken + val(1));
    let next = self.iter.shader_next();
    ShaderIterNext {
      has_next: next.has_next.and(taken.less_than(self.count)),
      item: next.item,
    }
  }

  fn check_iter_scope(&self) {
    self.scope.check();
    self.iter.check_iter_scope();
  }
}

pub struct ShaderFlatMapIter<Outer, Inner: ShaderIterState, F> {
  outer: Outer,
  /// the initial inner iterator is exhausted, it is reset by the state created from the outer item
  inner: Inner::AbstractLeftValue,
  f: F,
}

impl<Outer, Inner, F> ShaderIterator for ShaderFlatMapIter<Outer, Inner, F>
where
  Outer: ShaderIterator,
  Inner: ShaderIterState,
  <Inner::AbstractLeftValue as ShaderIterator>::Item: ShaderAbstractRightValue,
  F: Fn(Outer::Item) -> Inner,
{
  type Item = <Inner::AbstractLeftValue as ShaderIterator>::Item;

  fn shader_next(&self) -> ShaderIterNext<'_, Self::Item> {
    // poll the inner, if it is exhausted, poll the outer to reset the inner and retry, until the
    // inner has next or the outer is exhausted, so the empty inner is skipped
    ShaderIterNext::search(|cx, found| {
      let inner = self.inner.shader_next();
      if_by(inner.has_next, || found(inner.item()));

      let outer = self.outer.shader_next();
      if_by(outer.has_next.not(), || {
        cx.do_break();
      });
      self.inner.abstract_store((self.f)(outer.item()));
    })
  }

  fn check_iter_scope(&self) {
    self.outer.check_iter_scope();
    self.inner.check_iter_scope();
  }
}
