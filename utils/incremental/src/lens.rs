use std::marker::PhantomData;

use crate::*;

pub struct DeltaView<'a, T: IncrementalBase> {
  pub data: &'a T,
  pub delta: &'a T::Delta,
}

pub struct DeltaViewMut<'a, T: IncrementalBase> {
  pub data: &'a mut T,
  pub delta: &'a mut T::Delta,
}

pub trait DeltaLens<T: IncrementalBase, U: IncrementalBase> {
  fn map_delta(&self, delta: DeltaOf<U>, cb: &mut dyn FnMut(DeltaOf<T>));
  fn check_delta(&self, delta: DeltaView<T>, cb: &mut dyn FnMut(DeltaView<U>));
}

#[derive(Clone, Copy)]
pub struct FieldDelta<M, C> {
  map_delta: M,
  check_delta: C,
}

#[macro_export]
macro_rules! lens_d {
  ($ty:ty, $field:tt) => {
    $crate::FieldDelta::new::<$ty, _>(
      |inner_d, cb| cb(DeltaOf::<$ty>::$field(inner_d)),
      |v, cb| {
        if let DeltaOf::<$ty>::$field(inner_d) = v.delta {
          cb(DeltaView {
            data: &v.data.$field,
            delta: &inner_d,
          })
        }
      },
    )
  };
}

impl<M, C> FieldDelta<M, C> {
  pub fn new<T, U>(map_delta: M, check_delta: C) -> Self
  where
    T: IncrementalBase,
    U: IncrementalBase,
    M: Fn(DeltaOf<U>, &mut dyn FnMut(DeltaOf<T>)),
    C: Fn(DeltaView<T>, &mut dyn FnMut(DeltaView<U>)),
  {
    Self {
      map_delta,
      check_delta,
    }
  }
}

impl<T, U, M, C> DeltaLens<T, U> for FieldDelta<M, C>
where
  T: IncrementalBase,
  U: IncrementalBase,
  M: Fn(DeltaOf<U>, &mut dyn FnMut(DeltaOf<T>)),
  C: Fn(DeltaView<T>, &mut dyn FnMut(DeltaView<U>)),
{
  fn map_delta(&self, delta: DeltaOf<U>, cb: &mut dyn FnMut(DeltaOf<T>)) {
    (self.map_delta)(delta, cb)
  }

  fn check_delta(&self, delta: DeltaView<T>, cb: &mut dyn FnMut(DeltaView<U>)) {
    (self.check_delta)(delta, cb)
  }
}

pub struct DeltaChain<D1, D2, M> {
  d1: D1,
  middle: PhantomData<M>,
  d2: D2,
}

pub trait ChainDelta<T, M, U>: Sized
where
  T: IncrementalBase,
  M: IncrementalBase,
  U: IncrementalBase,
  Self: DeltaLens<M, U>,
{
  fn chain<N>(self, next: N) -> DeltaChain<N, Self, M>
  where
    N: DeltaLens<T, M>;
}

impl<T, M, U, X> ChainDelta<T, M, U> for X
where
  T: IncrementalBase,
  M: IncrementalBase,
  U: IncrementalBase,
  Self: DeltaLens<M, U>,
{
  fn chain<N>(self, next: N) -> DeltaChain<N, Self, M>
  where
    N: DeltaLens<T, M>,
  {
    DeltaChain::new(next, self)
  }
}

impl<D1: Clone, D2: Clone, M> Clone for DeltaChain<D1, D2, M> {
  fn clone(&self) -> Self {
    Self {
      d1: self.d1.clone(),
      middle: self.middle,
      d2: self.d2.clone(),
    }
  }
}
impl<D1: Copy, D2: Copy, M> Copy for DeltaChain<D1, D2, M> {}

impl<D1, D2, M> DeltaChain<D1, D2, M> {
  pub fn new(d1: D1, d2: D2) -> Self {
    Self {
      d1,
      middle: PhantomData,
      d2,
    }
  }
}

impl<T, M, U, D1, D2> DeltaLens<T, U> for DeltaChain<D1, D2, M>
where
  T: IncrementalBase,
  M: IncrementalBase,
  U: IncrementalBase,
  D1: DeltaLens<T, M>,
  D2: DeltaLens<M, U>,
{
  fn map_delta(&self, delta: DeltaOf<U>, cb: &mut dyn FnMut(DeltaOf<T>)) {
    self.d2.map_delta(delta, &mut |d| self.d1.map_delta(d, cb))
  }

  fn check_delta(&self, delta: DeltaView<T>, cb: &mut dyn FnMut(DeltaView<U>)) {
    self
      .d1
      .check_delta(delta, &mut |delta| self.d2.check_delta(delta, cb))
  }
}
