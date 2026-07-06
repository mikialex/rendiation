---
name: query-system
description: >
  Complete reference for the rendiation incremental query system (utility/query).
  Covers Query and MultiQuery traits, container implementations, operator combinators
  (map, filter, join, chain, union), the dual-query incremental model (DualQuery,
  DualQueryLike, ValueChange), fanout for FK-based change propagation, DataChanges
  for batch change processing, and all provided interfaces. Use when writing or
  understanding incremental computation pipelines that react to database changes.
metadata:
  version: "1.0"
  updated: "2026-05-28"
---

The `utility/query` crate provides an **incremental view maintenance engine** for
in-memory relational data. It formalizes the pattern of "current snapshot + change
sequence" and provides composable operators that keep both synchronized.

Key files:

| File | Purpose |
|------|---------|
| [utility/query/src/query/mod.rs](utility/query/src/query/mod.rs) | `Query` trait definition, blanket impls for `&T` and `Option<T>` |
| [utility/query/src/query/container.rs](utility/query/src/query/container.rs) | Base container impls (FastHashMap, FastHashSet, Arena, etc.) |
| [utility/query/src/query/operator/](utility/query/src/query/operator/) | Query combinators (map, filter, join, chain, union) |
| [utility/query/src/multi_query/mod.rs](utility/query/src/multi_query/mod.rs) | `MultiQuery` trait and base impls |
| [utility/query/src/multi_query/operator.rs](utility/query/src/multi_query/operator.rs) | MultiQuery combinators |
| [utility/query/src/multi_query/bookkeeping.rs](utility/query/src/multi_query/bookkeeping.rs) | Reverse-relation maintenance utilities |
| [utility/query/src/delta_query/mod.rs](utility/query/src/delta_query/mod.rs) | `DualQuery`, `DualQueryLike`, `TriQuery`, `TriQueryLike` |
| [utility/query/src/delta_query/delta.rs](utility/query/src/delta_query/delta.rs) | `ValueChange<V>` enum and merge/integrate/validate utilities |
| [utility/query/src/delta_query/fanout.rs](utility/query/src/delta_query/fanout.rs) | `fanout_impl` — FK-based incremental change propagation |
| [utility/query/src/delta_query/join.rs](utility/query/src/delta_query/join.rs) | `CrossJoinValueChange` for cross-join delta computation |
| [utility/query/src/delta_query/union.rs](utility/query/src/delta_query/union.rs) | `UnionValueChange` for union delta computation |
| [utility/query/src/delta_query/filter.rs](utility/query/src/delta_query/filter.rs) | `FilterMapQueryChange` for delta filtering |
| [utility/query/src/delta_query/map.rs](utility/query/src/delta_query/map.rs) | `ValueChangeMapper` and delta map combinators |
| [utility/query/src/delta_query/mutate_target.rs](utility/query/src/delta_query/mutate_target.rs) | `QueryMutationCollector` for auto-tracking mutations |
| [utility/query/src/delta_query/previous_view.rs](utility/query/src/delta_query/previous_view.rs) | `QueryPreviousView` for reconstructing previous state |
| [utility/query/src/change_query/mod.rs](utility/query/src/change_query/mod.rs) | `DataChanges` trait and `LinearBatchChanges` |
| [utility/query/src/change_query/delta_as_change.rs](utility/query/src/change_query/delta_as_change.rs) | `DeltaQueryAsChange` — bridge from Query<ValueChange> to DataChanges |
| [utility/query/src/lock_holder.rs](utility/query/src/lock_holder.rs) | Lock wrappers that implement Query/MultiQuery |
| [utility/query/src/utility/tree.rs](utility/query/src/utility/tree.rs) | `compute_tree_derive` for incremental tree derivation |

Import everything with:

```rust
use query::*;
```

## Core Concepts

### Query trait

A `Key → Value` mapping. The fundamental read-only data access abstraction.

```rust
pub trait Query: Send + Sync + Clone {
    type Key: CKey;       // Eq + Hash + Clone + Send + Sync + Debug + PartialEq + 'static
    type Value: CValue;   // Clone + Send + Sync + Debug + PartialEq + 'static

    fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_;
    fn access(&self, key: &Self::Key) -> Option<Self::Value>;

    /// May have false positives (return true when actually empty).
    /// False negatives are not allowed.
    fn has_item_hint(&self) -> bool;
}
```

**Base container implementations:** `FastHashMap<K, V>`, `Arc<FastHashMap<K, V>>`, `FastHashSet<K>` (value is `()`), `Arena<V>` (key is `u32`), `IndexReusedVec<V>` (key is `u32`), `IndexKeptVec<V>` (key is `u32`), `IdenticalCollection<V>` (all keys return same value), `EmptyQuery<K, V>` (always empty), `KeptQuery<T>` (delegates to inner while holding an `Arc<dyn Any>`).

### MultiQuery trait

A `Key → Set<Value>` (one-to-many) mapping. 

```rust
pub trait MultiQuery: Send + Sync + Clone {
    type Key: CKey;
    type Value: CValue;

    fn iter_keys(&self) -> impl Iterator<Item = Self::Key> + '_;

    /// Returns None if key is not in the query at all.
    /// Returns Some(empty iterator) if key exists but maps to no values.
    fn access_multi(&self, key: &Self::Key) -> Option<impl Iterator<Item = Self::Value> + '_>;
}
```

**Base container:** `FastHashMap<K, FastHashSet<V>>`.

### ValueChange<V>

Describes a single key's atomic change. The building block of delta queries.

```rust
pub enum ValueChange<V> {
    Delta(V, Option<V>),   // (new_value, Option<old_value>)
    Remove(V),             // (old_value)
}
```

- `Delta(v, None)` = new insert
- `Delta(v, Some(old))` = update from old to v
- `Remove(old)` = deletion

Key methods: `new_value()`, `old_value()`, `into_new_value()`, `is_removed()`, `is_new_insert()`, `is_redundant()` (returns true if Delta with v == old), `merge(&mut self, new)` (collapses two sequential changes into one), `map(mapper)`.

Utility functions: `merge_change` (merge a change into a FastHashMap of mutations), `integrate_change` (apply a change to state), `make_checker` (lift a `Fn(V) -> Option<V2>` to work on `ValueChange<V>`), `validate_delta` (apply delta to state with assertions).

### DualQuery and DualQueryLike

The core incremental abstraction. A `DualQuery` pairs a **view** (current full snapshot) with a **delta** (recent changes). All combinators operate on both simultaneously, so derived queries automatically produce correct deltas.

```rust
#[derive(Clone)]
pub struct DualQuery<T, U> {
    pub view: T,    // Query<Key=K, Value=V>
    pub delta: U,   // Query<Key=K, Value=ValueChange<V>>
}

pub trait DualQueryLike: Send + Sync + Clone + 'static {
    type Key: CKey;
    type Value: CValue;
    type View: Query<Key = Self::Key, Value = Self::Value>;
    type Delta: Query<Key = Self::Key, Value = ValueChange<Self::Value>>;

    fn view_delta(self) -> (Self::View, Self::Delta);
    fn view_delta_ref(&self) -> (&Self::View, &Self::Delta);
}
```

### TriQuery and TriQueryLike

A `TriQuery` extends `DualQuery` with a **reverse multi-query** — the inverse of a 1:1 relation. This is what enables `fanout`.

```rust
pub struct TriQuery<T, U, V> {
    pub base: DualQuery<T, U>,
    pub rev_many_view: V,  // MultiQuery<Key = Value, Value = Key>
}

pub trait TriQueryLike: DualQueryLike<Value: CKey> {
    type InvView: MultiQuery<Key = Self::Value, Value = Self::Key>;
    fn inv_view_view_delta(self) -> (Self::InvView, Self::View, Self::Delta);
}
```

In practice, `TriQuery` is always created from a database foreign key via `cx.use_db_rev_ref_tri_view::<ForeignKey>()`. The FK stores "many-side → one-side", and the TriQuery adds the reverse "one-side → set-of-many-side".

### DataChanges trait

A simpler batch-change abstraction that separates removes from updates and doesn't track previous values.

```rust
pub trait DataChanges: Send + Sync + Clone {
    type Key: CKey;
    type Value;
    fn has_change(&self) -> bool;
    fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_;
    fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_;
}
```

Used as a lighter-weight alternative to `Query<Value=ValueChange<V>>` when previous values aren't needed. `DeltaQueryAsChange<T>` bridges from a delta query to DataChanges.

---

## How Data Flows (Incremental Pipeline)

```
Database writes
    ↓
Change detect & capture
    ↓
DBDualQuery<T> = DualQuery<DBView<T>, DBDelta<T>>   ← produced by use_dual_query::<T>()
    ↓
Combinator chain (map, filter, fanout, union, zip, materialize...)
    ↓
Consume composed changes
    ↓
GPU resource update / render pass dispatch
```

The key insight: **fanout is the primary mechanism for following FK relationships**.
A component keyed on entity type A needs to be used keyed on entity type B.
Fanout uses a TriQuery (which knows the FK mapping and its inverse) to propagate changes across the relationship.

A typical pipeline:
```rust
// Node-local matrices (keyed on SceneNodeEntity)
let node_mats = use_global_node_world_mat(cx);

// Fanout to SceneModelEntity via SceneModelRefNode FK
let model_mats = node_mats.fanout(cx.use_db_rev_ref_tri_view::<SceneModelRefNode>(), cx);

// Further fanout to SceneModelEntity via SceneModelStdModelRenderPayload FK
let std_model_mats = model_mats.fanout(cx.use_db_rev_ref_tri_view::<SceneModelStdModelRenderPayload>(), cx);
```

---

## API Reference

### Query Combinators (via `QueryExt` trait)

| Method | Returns | Description |
|--------|---------|-------------|
| `.map_value(f: Fn(V) -> V2)` | `MappedValueQuery` | Map only the value |
| `.map(f: Fn(&K, V) -> V2)` | `MappedQuery` | Map with key access |
| `.filter_map(f: Fn(V) -> Option<V2>)` | `FilterMapQuery` | Filter and map values |
| `.key_dual_map(f1, f2)` | `KeyDualMappedQuery` | Bidirectional key type conversion |
| `.key_dual_map_partial(f1, f2)` | `KeyDualMappedQuery` | Bidirectional key conversion (partial reverse) |
| `.chain(next: Q)` | `ChainQuery` | Compose: first.value → next.key lookup |
| `.into_boxed()` | `Arc<dyn DynQuery>` | Type-erase to dynamic dispatch |

### MultiQuery Combinators (via `MultiQueryExt` trait)

| Method | Returns | Description |
|--------|---------|-------------|
| `.multi_map(f: Fn(V) -> V2)` | `MappedValueQuery` | Map each value |
| `.multi_key_dual_map(f1, f2)` | `KeyDualMappedQuery` | Bidirectional key conversion |
| `.into_boxed_multi()` | `Box<dyn DynMultiQuery>` | Type-erase |

### Delta Query Combinators (via `DeltaQueryExt` trait)

Applies to any `Query<Value = ValueChange<V>>`:

| Method | Returns | Description |
|--------|---------|-------------|
| `.delta_map(f: Fn(&K, V) -> V2)` | `MappedQuery` | Map values preserving delta structure |
| `.delta_map_value(f: Fn(V) -> V2)` | `MappedValueQuery` | Map values (no key access) |
| `.delta_filter_map(f: Fn(V) -> Option<V2>)` | `FilterMapQueryChange` | Filter/map on delta values |

### DualQueryLike Combinators

| Method | Returns | Description |
|--------|---------|-------------|
| `.dual_query_map(f)` | `impl DualQueryLike` | Map values on both view and delta |
| `.dual_query_map_kv(f)` | `impl DualQueryLike` | Map with key access on both |
| `.dual_query_filter(f)` | `impl DualQueryLike` | Filter on both |
| `.dual_query_filter_map(f)` | `impl DualQueryLike` | Filter+map on both |
| `.dual_query_union(other, f)` | `impl DualQueryLike` | Union two queries by key with merge fn |
| `.dual_query_select(other)` | `impl DualQueryLike` | Union with mutual exclusion (disjoint key sets) |
| `.dual_query_zip(other)` | `impl DualQueryLike` | Strict zip (both must have same keys) |
| `.dual_query_intersect(other)` | `impl DualQueryLike` | Intersection (both keys required) |
| `.dual_query_filter_by_set(other)` | `impl DualQueryLike` | Keep only keys present in other |
| `.dual_query_cross_join(other)` | `impl DualQueryLike` | Cartesian product of key spaces |
| `.fanout(tri_query)` | `DualQuery` | FK-based incremental propagation (see below) |
| `.view()` | `Self::View` | Extract view component |
| `.delta()` | `Self::Delta` | Extract delta component |
| `.materialize_delta()` | `DualQuery` | Force materialize delta to hashmap |
| `.into_boxed()` | `BoxedDynDualQuery` | Type-erase |
| `.has_delta_hint()` | `bool` | Quick check for pending changes |

### fanout

The most important and complex combinator. Signature:

```rust
fn fanout<R: TriQueryLike<Value = Self::Key>>(self, other: R)
    -> DualQuery<ChainQuery<R::View, Self::View>, Arc<FastHashMap<R::Key, ValueChange<Self::Value>>>>
```

Input:
- `self`: upstream data keyed on A (view + delta of X values)
- `other`: a `TriQuery` for the A↔B relationship (FK view + delta + reverse multi-index)

Output: data keyed on B (view = chain through FK, delta = materialized incremental changes)

The delta computation runs in two phases:
1. **Relational changes**: when the FK mapping changes (B→A), look up the new/old X values
2. **Upstream value changes**: when X changes, fan out through reverse relation to affected B keys. Entries that cancel with Phase 1 results are removed.

### Lock Helpers

| Type | Description |
|------|-------------|
| `LockReadGuardHolder<T>` | Read lock guard that implements `Query` and `MultiQuery` |
| `LockWriteGuardHolder<T>` | Write lock guard (can downgrade to read) |
| `MutexGuardHolder<T>` | Mutex guard with Deref/DerefMut |

### Mutation Helpers

| Type / Function | Description |
|-----------------|-------------|
| `QueryLikeMutateTarget<K, V>` | Trait for mutable key-value stores |
| `QueryMutationCollector<D, T>` | Wraps a target + delta store; auto-records changes on mutation |

### Reverse Relation Bookkeeping

| Function | Description |
|----------|-------------|
| `bookkeeping_hash_relation(mapping, changes)` | Maintain `FastHashMap<V, FastHashSet<K>>` from delta |
| `bookkeeping_dense_index_relation(mapping, changes)` | Same but for `DenseIndexMapping` (small-list→hashset adaptive) |

### Validation

| Function | Description |
|----------|-------------|
| `validate_query_consistency(q)` | Verify iter_key_value/access/has_item_hint consistency |
| `validate_multi_query_consistency(q)` | Verify iter_keys/access_multi consistency |

## Key Traits (for custom implementations)

| Trait | Where to impl | Purpose |
|-------|---------------|---------|
| `CKey` | Any Eq+Hash+CValue type | Marker for valid key types |
| `CValue` | Any Clone+Send+Sync+Debug+PartialEq+'static | Marker for valid value types |
| `Query` | Your container | Key→Value read access |
| `MultiQuery` | Your container | Key→Set<Value> read access |
| `DualQueryLike` | Rarely needed (2 built-in impls) | Dual view+delta abstraction |
| `TriQueryLike` | Rarely needed (1 built-in impl) | Dual + reverse multi-index |
| `DataChanges` | Batch change container | Separated remove/insert iteration |
| `IteratorProvider` | `[T; N]`, `Vec<T>` | Enable `Select`/`SelectChanges` over collections |
| `QueryLikeMutateTarget` | Mutable store | Enable `QueryMutationCollector` tracking |
| `DynQuery` / `DynValueRefQuery` | Auto-implemented via blanket | Dynamic dispatch for Query |
| `DynMultiQuery` | Auto-implemented via blanket | Dynamic dispatch for MultiQuery |
