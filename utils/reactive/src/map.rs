use std::collections::HashMap;

use futures::{stream::FuturesUnordered, *};

use crate::do_updates;

pub trait ReactiveDerived: Sized {
  type Source;
  type ChangeStream: Stream + Unpin;
  type DropFuture: Future<Output = ()> + Unpin;
  type Ctx;

  fn key(source: &Self::Source) -> usize;

  fn build(source: &Self::Source, ctx: &Self::Ctx) -> (Self, Self::ChangeStream, Self::DropFuture);

  fn update(&mut self, source: &Self::Source, change: &mut Self::ChangeStream, ctx: &Self::Ctx);
}

pub struct ReactiveMap<T: ReactiveDerived> {
  mapping: HashMap<usize, (T, T::ChangeStream)>,
  /// when drop consumed, we remove the mapped from mapping, we could make this sync to drop.
  /// but if we do so, the mapping have to wrapped in interior mutable container, and it's
  /// impossible to get mut reference directly in safe rust.
  ///
  /// user should call cleanup periodically to do the actually remove now.
  drop_futures: FuturesUnordered<KeyedDropFuture<T::DropFuture, usize>>,
}

impl<T: ReactiveDerived> Default for ReactiveMap<T> {
  fn default() -> Self {
    Self {
      mapping: Default::default(),
      drop_futures: Default::default(),
    }
  }
}

type KeyedDropFuture<F: Future<Output = ()>, T> = impl Future<Output = T>;
fn map_drop_future<T, F: Future<Output = ()>>(f: F, key: T) -> KeyedDropFuture<F, T> {
  f.map(|_| key)
}

impl<T: ReactiveDerived> ReactiveMap<T> {
  pub fn get_with_update(&mut self, source: &T::Source, ctx: &T::Ctx) -> &mut T {
    let id = T::key(source);

    let (mapped, changes) = self.mapping.entry(id).or_insert_with(|| {
      let (mapped, stream, future) = T::build(source, ctx);
      self.drop_futures.push(map_drop_future(future, id));
      (mapped, stream)
    });

    mapped.update(source, changes, ctx);
    mapped
  }

  pub fn cleanup(&mut self) {
    do_updates(&mut self.drop_futures, |id| {
      self.mapping.remove(&id);
    })
  }
}
