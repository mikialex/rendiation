#![feature(hash_raw_entry)]

use std::{hash::Hash, sync::Arc};

use fast_hash_collection::FastHashMap;
use parking_lot::RwLock;

pub struct ReuseKVPoolInternal<K, V> {
  enable_reusing: bool,
  max_live_tick: u32,
  pool: FastHashMap<K, Vec<(V, u32)>>,
  creator: Box<dyn Fn(&K) -> V + Send + Sync>,
}

impl<K: Clone + Eq + Hash, V> ReuseKVPoolInternal<K, V> {
  pub fn request(&mut self, k: &K) -> V {
    let (k, v) = self
      .pool
      .raw_entry_mut()
      .from_key(k)
      .or_insert_with(|| (k.clone(), Vec::new()));

    if let Some((v, _)) = v.pop() {
      v
    } else {
      (self.creator)(k)
    }
  }
}

pub struct ReuseKVPool<K, V> {
  internal: Arc<RwLock<ReuseKVPoolInternal<K, V>>>,
}

impl<K, V> Clone for ReuseKVPool<K, V> {
  fn clone(&self) -> Self {
    Self {
      internal: self.internal.clone(),
    }
  }
}

impl<K: Clone + Eq + Hash, V> ReuseKVPool<K, V> {
  pub fn request(&self, k: &K) -> ReuseableItem<K, V> {
    let mut internal = self.internal.write();
    let v = internal.request(k);
    ReuseableItem {
      pool: self.clone(),
      key: k.clone(),
      item: Some(v),
    }
  }
}

impl<K, V> ReuseKVPool<K, V> {
  pub fn new(creator: impl Fn(&K) -> V + Send + Sync + 'static) -> Self {
    Self {
      internal: Arc::new(RwLock::new(ReuseKVPoolInternal {
        enable_reusing: true,
        max_live_tick: 3,
        pool: Default::default(),
        creator: Box::new(creator),
      })),
    }
  }

  pub fn with_enable_reusing(self, enable_reusing: bool) -> Self {
    self.set_enable_reusing(enable_reusing);
    self
  }

  pub fn set_enable_reusing(&self, enable_reusing: bool) {
    let mut internal = self.internal.write();
    internal.enable_reusing = enable_reusing;
    if !enable_reusing {
      internal.pool.clear();
    }
  }

  pub fn with_max_live_tick(self, max_live_tick: u32) -> Self {
    self.set_max_live_tick(max_live_tick);
    self
  }

  pub fn set_max_live_tick(&self, max_live_tick: u32) {
    let mut internal = self.internal.write();
    let should_clean = internal.max_live_tick > max_live_tick;
    internal.max_live_tick = max_live_tick;
    if should_clean {
      drop(internal);
      self.tick();
    }
  }

  pub fn clear_all_cached(&self) {
    let mut internal = self.internal.write();
    internal.pool.clear();
  }

  /// remove all item that not been used for given max tick time
  pub fn tick(&self) {
    let mut internal = self.internal.write();
    let max_live_tick = internal.max_live_tick;
    for v in internal.pool.values_mut() {
      let mut p = v.len();
      while p > 0 {
        p -= 1;
        let (_, tick) = v[p];
        if tick < max_live_tick {
          continue;
        } else {
          v.swap_remove(p);
        }
      }
    }
  }
}

pub struct ReuseableItem<K: Eq + Hash + Clone, V> {
  pool: ReuseKVPool<K, V>,
  key: K,
  item: Option<V>,
}

impl<K: Eq + Hash + Clone, V> ReuseableItem<K, V> {
  pub fn item(&self) -> &V {
    self.item.as_ref().unwrap()
  }
}

impl<K: Eq + Hash + Clone, V> Drop for ReuseableItem<K, V> {
  fn drop(&mut self) {
    let mut pool = self.pool.internal.write();
    if pool.enable_reusing {
      let pool = pool
        .pool
        .entry(self.key.clone()) // maybe not exist when entire pool cleared when resize
        .or_default();
      pool.push((self.item.take().unwrap(), 0));
    } // else drop V directly
  }
}
