#![feature(type_alias_impl_trait)]

use std::{
  future::Future,
  io::{Read, Write},
  path::PathBuf,
  task::{Context, Poll},
};

use futures::{stream::FuturesUnordered, FutureExt, Stream};
use query::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

mod device_usage_counter;
pub use device_usage_counter::*;

mod texture;
pub use texture::*;

pub enum MaybePersistData<T: SwappableData> {
  Offline {
    key: u64,
    meta: T::Meta,
  },
  /// Keep it living if some feature requires it.
  Persist(T),
}

pub trait SwappableData {
  type Meta;
  fn swap_out(self, writer: &dyn Write) -> Self::Meta;
  fn swap_in(read: &dyn Read) -> Self;
}

pub struct DiskSwapManager {
  write_folder_root: PathBuf,
}

impl DiskSwapManager {
  pub fn load<T: SwappableData>(&self, id: u64) -> Box<dyn Future<Output = Option<T>>> {
    let file_path = self.write_folder_root.join(id.to_string());
    let buffer = std::fs::read(&file_path).unwrap();
    // let data = T::swap_in(&buffer);
    todo!()
  }
}

pub trait ScheduleSource<K> {
  type LoadingFuture: Future<Output = ()>;
  /// should be immutable
  fn retain_cost(&self, item: &K) -> u64;
  /// should be immutable
  fn loading_peak_cost(&self, item: &K) -> u64;
  fn load_data(&self, item: &K) -> Self::LoadingFuture;
  fn unload_data(&self, item: &K);
}

pub struct Scheduler<K, S: ScheduleSource<K>> {
  retain_capacity_limitation: u64,
  source: S,
  current_loading_budget: u64,
  loading: FuturesUnordered<WrapFuture<S::LoadingFuture, K>>,
  loading_set: fast_hash_collection::FastHashSet<K>,
  current_living_set: fast_hash_collection::FastHashSet<K>,
}

type WrapFuture<T: Future<Output = ()>, K> = impl Future<Output = K>;
#[define_opaque(WrapFuture)]
fn wrap_future<K, T: Future<Output = ()>>(f: T, k: K) -> WrapFuture<T, K> {
  f.map(|_| k)
}

impl<K: CKey, S: ScheduleSource<K>> Scheduler<K, S> {
  pub fn new(source: S, retain_capacity_limitation: u64, loading_limitation: u64) -> Self {
    Self {
      retain_capacity_limitation,
      current_loading_budget: loading_limitation,
      source,
      loading_set: Default::default(),
      loading: Default::default(),
      current_living_set: Default::default(),
    }
  }

  pub fn poll_loading(&mut self, cx: &mut Context) {
    let loading = std::pin::pin!(&mut self.loading);
    if let Poll::Ready(Some(r)) = loading.poll_next(cx) {
      self.current_loading_budget += self.source.retain_cost(&r);
      self.loading_set.remove(&r);
      self.current_living_set.insert(r);
    }
  }

  // todo, the current implementation is not incremental
  pub fn schedule(&mut self, weights: impl Query<Key = K, Value = f32>) {
    let mut weights_list = weights.iter_key_value().collect::<Vec<_>>();

    weights_list.sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).expect("unexpected bad weight"));

    let mut budget = self.retain_capacity_limitation;
    let mut fit_count = 0;
    for (k, _) in &weights_list {
      let cost = self.source.retain_cost(k);
      if budget >= cost {
        fit_count += 1;
        budget -= cost;
      }
    }

    for (k, _) in &weights_list[fit_count..weights_list.len()] {
      if self.current_living_set.remove(k) {
        self.source.unload_data(k);
      }
      // note, we not remove the loading set, because even if the future drop triggers cancellation,
      // the cancellation may not effect immediately
    }

    for (k, _) in &weights_list[0..fit_count] {
      if self.current_living_set.contains(k) {
        continue;
      }
      if self.loading_set.contains(k) {
        continue;
      }

      let cost = self.source.retain_cost(k);
      if self.current_loading_budget >= cost {
        self.loading_set.insert(k.clone());
        self
          .loading
          .push(wrap_future(self.source.load_data(k), k.clone()));
        self.current_loading_budget -= cost;
      }
    }
  }
}
