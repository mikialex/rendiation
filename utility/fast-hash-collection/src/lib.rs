// https://nnethercote.github.io/perf-book/hashing.html

pub type FastHasher = rustc_hash::FxHasher;
pub type FastHasherBuilder = std::hash::BuildHasherDefault<FastHasher>;
pub type FastHashMap<K, V> = hashbrown::HashMap<K, V, FastHasherBuilder>;
pub type FastHashSet<K> = hashbrown::HashSet<K, FastHasherBuilder>;
