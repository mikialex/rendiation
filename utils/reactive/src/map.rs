use std::collections::HashMap;

use futures::{stream::FuturesUnordered, *};

use crate::do_updates;

// pub trait ReactiveSource {
//   type ChangeStream: Stream<Item = ()> + Unpin;
//   type DropFuture: Future<Output = ()> + Unpin;
//   type Ctx;
// }

pub trait ReactiveMapping: Sync + Send {
  type Mapped;
  type ChangeStream: Stream<Item = ()> + Unpin;
  type DropFuture: Future<Output = ()> + Unpin;
  type Ctx;

  fn id(&self) -> usize;

  fn build(&self, ctx: &Self::Ctx) -> (Self::Mapped, Self::ChangeStream, Self::DropFuture);

  fn update(&self, mapped: &mut Self::Mapped, change: &mut Self::ChangeStream, ctx: &Self::Ctx);
}

pub struct ReactiveMap<T: ReactiveMapping> {
  mapping: HashMap<usize, (T::Mapped, T::ChangeStream)>,
  /// when drop consumed, we remove the mapped from mapping, we could make this sync to drop.
  /// but if we do so, the mapping have to wrapped in interior mutable container, and it's
  /// impossible to get mut reference directly in safe rust.
  ///
  /// user should call cleanup periodically to do the actually remove now.
  drop_futures: FuturesUnordered<KeyedDropFuture<T::DropFuture, usize>>,
}

impl<T: ReactiveMapping> Default for ReactiveMap<T> {
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

impl<T: ReactiveMapping> ReactiveMap<T> {
  pub fn get_with_update(&mut self, source: &T, ctx: &T::Ctx) -> &mut T::Mapped {
    let id = source.id();

    let (gpu_resource, changes) = self.mapping.entry(id).or_insert_with(|| {
      let (mapped, stream, future) = T::build(source, ctx);
      self.drop_futures.push(map_drop_future(future, id));
      (mapped, stream)
    });

    source.update(gpu_resource, changes, ctx);
    gpu_resource
  }

  pub fn cleanup(&mut self) {
    do_updates(&mut self.drop_futures, |id| {
      self.mapping.remove(&id);
    })
  }
}
