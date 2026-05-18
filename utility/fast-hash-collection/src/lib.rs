// https://nnethercote.github.io/perf-book/hashing.html

use std::hash::Hasher;

pub type FastHasher = rustc_hash::FxHasher;
pub type FastHasherBuilder = std::hash::BuildHasherDefault<FastHasher>;
pub type FastHashMap<K, V> = hashbrown::HashMap<K, V, FastHasherBuilder>;
pub type FastHashSet<K> = hashbrown::HashSet<K, FastHasherBuilder>;

#[inline(always)]
pub fn fast_hash_scope(f: impl FnOnce(&mut FastHasher)) -> u64 {
  let mut hasher = FastHasher::default();
  f(&mut hasher);
  hasher.finish()
}
