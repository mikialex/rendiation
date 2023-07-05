// https://nnethercote.github.io/perf-book/hashing.html

pub type FastHashMap<K, V> = rustc_hash::FxHashMap<K, V>;
pub type FastHashSet<K> = rustc_hash::FxHashSet<K>;
pub type FastHasher = rustc_hash::FxHasher;
