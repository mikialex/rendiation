---
name: query-system
description: >
  rendiation 增量查询系统(utility/query)的完整参考文档。
  涵盖 Query 与 MultiQuery trait、容器实现、算子组合器(map、filter、join、chain、union)、
  双查询增量模型(DualQuery、DualQueryLike、ValueChange)、用于基于外键的
  变更传播的扇出(fanout)、用于批量变更处理的 DataChanges,以及所有提供的接口。
  在编写或理解响应数据库变更的增量计算管线时使用。
metadata:
  version: "1.0"
  updated: "2026-05-28"
---

`utility/query` crate 为内存中的关系数据提供了**增量视图维护引擎**。它形式化了"当前快照 + 变更序列"的模式,并提供可组合的算子,使两者保持同步。

关键文件:

| 文件 | 用途 |
|------|---------|
| [utility/query/src/query/mod.rs](../../../../../rendiation/utility/query/src/query/mod.rs) | `Query` trait 定义,`&T` 与 `Option<T>` 的全覆盖实现 |
| [utility/query/src/query/container.rs](../../../../../rendiation/utility/query/src/query/container.rs) | 基础容器实现(FastHashMap、FastHashSet、Arena 等) |
| [utility/query/src/query/operator/](../../../../../rendiation/utility/query/src/query/operator/) | 查询组合算子(map、filter、join、chain、union) |
| [utility/query/src/multi_query/mod.rs](../../../../../rendiation/utility/query/src/multi_query/mod.rs) | `MultiQuery` trait 与基础实现 |
| [utility/query/src/multi_query/operator.rs](../../../../../rendiation/utility/query/src/multi_query/operator.rs) | MultiQuery 组合算子 |
| [utility/query/src/multi_query/bookkeeping.rs](../../../../../rendiation/utility/query/src/multi_query/bookkeeping.rs) | 反向关系维护工具 |
| [utility/query/src/delta_query/mod.rs](../../../../../rendiation/utility/query/src/delta_query/mod.rs) | `DualQuery`、`DualQueryLike`、`TriQuery`、`TriQueryLike` |
| [utility/query/src/delta_query/delta.rs](../../../../../rendiation/utility/query/src/delta_query/delta.rs) | `ValueChange<V>` 枚举及 merge/integrate/validate 工具 |
| [utility/query/src/delta_query/fanout.rs](../../../../../rendiation/utility/query/src/delta_query/fanout.rs) | `fanout_impl` — 基于外键的增量变更传播 |
| [utility/query/src/delta_query/join.rs](../../../../../rendiation/utility/query/src/delta_query/join.rs) | `CrossJoinValueChange` — 交叉连接增量计算 |
| [utility/query/src/delta_query/union.rs](../../../../../rendiation/utility/query/src/delta_query/union.rs) | `UnionValueChange` — 并集增量计算 |
| [utility/query/src/delta_query/filter.rs](../../../../../rendiation/utility/query/src/delta_query/filter.rs) | `FilterMapQueryChange` — 增量过滤 |
| [utility/query/src/delta_query/map.rs](../../../../../rendiation/utility/query/src/delta_query/map.rs) | `ValueChangeMapper` 与增量 map 组合算子 |
| [utility/query/src/delta_query/mutate_target.rs](../../../../../rendiation/utility/query/src/delta_query/mutate_target.rs) | `QueryMutationCollector` — 自动追踪变更 |
| [utility/query/src/delta_query/previous_view.rs](../../../../../rendiation/utility/query/src/delta_query/previous_view.rs) | `QueryPreviousView` — 重建前一状态 |
| [utility/query/src/change_query/mod.rs](../../../../../rendiation/utility/query/src/change_query/mod.rs) | `DataChanges` trait 与 `LinearBatchChanges` |
| [utility/query/src/change_query/delta_as_change.rs](../../../../../rendiation/utility/query/src/change_query/delta_as_change.rs) | `DeltaQueryAsChange` — 从 Query<ValueChange> 到 DataChanges 的桥梁 |
| [utility/query/src/lock_holder.rs](../../../../../rendiation/utility/query/src/lock_holder.rs) | 实现 Query/MultiQuery 的锁包装 |
| [utility/query/src/utility/tree.rs](../../../../../rendiation/utility/query/src/utility/tree.rs) | `compute_tree_derive` — 增量树推导 |

通过以下方式导入全部内容:

```rust
use query::*;
```

## 核心概念

### Query trait

一个 `Key → Value`(键 → 值)映射。这是最基本的只读数据访问抽象。

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

**基础容器实现:**`FastHashMap<K, V>`、`Arc<FastHashMap<K, V>>`、`FastHashSet<K>`(值为 `()`)、`Arena<V>`(键为 `u32`)、`IndexReusedVec<V>`(键为 `u32`)、`IndexKeptVec<V>`(键为 `u32`)、`IdenticalCollection<V>`(所有键返回相同值)、`EmptyQuery<K, V>`(恒为空)、`KeptQuery<T>`(持有 `Arc<dyn Any>` 时委托给内部实现)。

### MultiQuery trait

一个 `Key → Set<Value>`(键 → 值集合,一对多)映射。

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

**基础容器:**`FastHashMap<K, FastHashSet<V>>`。

### ValueChange<V>

描述单个键的原子变更。是增量(delta)查询的构建块。

```rust
pub enum ValueChange<V> {
    Delta(V, Option<V>),   // (new_value, Option<old_value>)
    Remove(V),             // (old_value)
}
```

- `Delta(v, None)` = 新插入
- `Delta(v, Some(old))` = 从 old 更新为 v
- `Remove(old)` = 删除

关键方法:`new_value()`、`old_value()`、`into_new_value()`、`is_removed()`、`is_new_insert()`、`is_redundant()`(当 Delta 的 v == old 时返回 true)、`merge(&mut self, new)`(将两次连续变更折叠为一次)、`map(mapper)`。

工具函数:`merge_change`(将变更合并到变更的 FastHashMap 中)、`integrate_change`(将变更应用到状态)、`make_checker`(将 `Fn(V) -> Option<V2>` 提升为可作用于 `ValueChange<V>`)、`validate_delta`(在断言下将增量应用到状态)。

### DualQuery 与 DualQueryLike

核心增量抽象。`DualQuery` 将**视图 view**(当前完整快照)与**增量 delta**(最近变更)配对。所有组合算子同时作用于两者,因此派生查询会自动产生正确的增量。

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

### TriQuery 与 TriQueryLike

`TriQuery` 在 `DualQuery` 之上增加了一个**反向多查询**——即 1:1 关系的逆。这正是 `fanout`(扇出)得以实现的基础。

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

实践中,`TriQuery` 总是通过 `cx.use_db_rev_ref_tri_view::<ForeignKey>()` 从数据库外键创建。外键存储"多侧 → 一侧"方向,而 TriQuery 补充了反向的"一侧 → 多侧集合"。

### DataChanges trait

一种更简单的批量变更抽象,将移除与更新分开处理,且不追踪旧值。

```rust
pub trait DataChanges: Send + Sync + Clone {
    type Key: CKey;
    type Value;
    fn has_change(&self) -> bool;
    fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_;
    fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_;
}
```

当不需要旧值时,可作为 `Query<Value=ValueChange<V>>` 的更轻量替代。`DeltaQueryAsChange<T>` 在增量查询与 DataChanges 之间建立桥梁。

---

## 数据如何流动(增量管线)

```
数据库写入
    ↓
变更检测与捕获
    ↓
DBDualQuery<T> = DualQuery<DBView<T>, DBDelta<T>>   ← 由 use_dual_query::<T>() 产生
    ↓
组合算子链(map、filter、fanout、union、zip、materialize...)
    ↓
消费组合后的变更
    ↓
GPU 资源更新 / 渲染通道派发
```

关键要点:**fanout(扇出)是沿外键关系传播的主要机制**。以实体类型 A 为键的组件需要被以实体类型 B 为键使用。fanout 借助 TriQuery(它同时知道外键映射及其逆)跨关系传播变更。

一个典型的管线:

```rust
// Node-local matrices (keyed on SceneNodeEntity)
let node_mats = use_global_node_world_mat(cx);

// Fanout to SceneModelEntity via SceneModelRefNode FK
let model_mats = node_mats.fanout(cx.use_db_rev_ref_tri_view::<SceneModelRefNode>(), cx);

// Further fanout to SceneModelEntity via SceneModelStdModelRenderPayload FK
let std_model_mats = model_mats.fanout(cx.use_db_rev_ref_tri_view::<SceneModelStdModelRenderPayload>(), cx);
```

---

## API 参考

### 查询组合算子(通过 `QueryExt` trait)

| 方法 | 返回值 | 说明 |
|--------|---------|-------------|
| `.map_value(f: Fn(V) -> V2)` | `MappedValueQuery` | 只映射值 |
| `.map(f: Fn(&K, V) -> V2)` | `MappedQuery` | 带键访问的映射 |
| `.filter_map(f: Fn(V) -> Option<V2>)` | `FilterMapQuery` | 过滤并映射值 |
| `.key_dual_map(f1, f2)` | `KeyDualMappedQuery` | 双向键类型转换 |
| `.key_dual_map_partial(f1, f2)` | `KeyDualMappedQuery` | 双向键转换(部分反向) |
| `.chain(next: Q)` | `ChainQuery` | 组合:first.value → next.key 查找 |
| `.into_boxed()` | `Arc<dyn DynQuery>` | 类型擦除为动态派发 |

### MultiQuery 组合算子(通过 `MultiQueryExt` trait)

| 方法 | 返回值 | 说明 |
|--------|---------|-------------|
| `.multi_map(f: Fn(V) -> V2)` | `MappedValueQuery` | 映射每个值 |
| `.multi_key_dual_map(f1, f2)` | `KeyDualMappedQuery` | 双向键转换 |
| `.into_boxed_multi()` | `Box<dyn DynMultiQuery>` | 类型擦除 |

### 增量查询组合算子(通过 `DeltaQueryExt` trait)

适用于任何 `Query<Value = ValueChange<V>>`:

| 方法 | 返回值 | 说明 |
|--------|---------|-------------|
| `.delta_map(f: Fn(&K, V) -> V2)` | `MappedQuery` | 保持增量结构映射值 |
| `.delta_map_value(f: Fn(V) -> V2)` | `MappedValueQuery` | 映射值(无键访问) |
| `.delta_filter_map(f: Fn(V) -> Option<V2>)` | `FilterMapQueryChange` | 对增量值进行过滤/映射 |

### DualQueryLike 组合算子

| 方法 | 返回值 | 说明 |
|--------|---------|-------------|
| `.dual_query_map(f)` | `impl DualQueryLike` | 同时在视图与增量上映射值 |
| `.dual_query_map_kv(f)` | `impl DualQueryLike` | 同时在两者上带键访问映射 |
| `.dual_query_filter(f)` | `impl DualQueryLike` | 同时在两者上过滤 |
| `.dual_query_filter_map(f)` | `impl DualQueryLike` | 同时在两者上过滤+映射 |
| `.dual_query_union(other, f)` | `impl DualQueryLike` | 用合并函数按键并集两个查询 |
| `.dual_query_select(other)` | `impl DualQueryLike` | 互斥并集(键集不相交) |
| `.dual_query_zip(other)` | `impl DualQueryLike` | 严格 zip(两者必须具有相同键) |
| `.dual_query_intersect(other)` | `impl DualQueryLike` | 交集(两者都要求有该键) |
| `.dual_query_filter_by_set(other)` | `impl DualQueryLike` | 只保留 other 中存在的键 |
| `.dual_query_cross_join(other)` | `impl DualQueryLike` | 键空间的笛卡尔积 |
| `.fanout(tri_query)` | `DualQuery` | 基于外键的增量传播(见下文) |
| `.view()` | `Self::View` | 提取视图组件 |
| `.delta()` | `Self::Delta` | 提取增量组件 |
| `.materialize_delta()` | `DualQuery` | 强制将增量物化为哈希表 |
| `.into_boxed()` | `BoxedDynDualQuery` | 类型擦除 |
| `.has_delta_hint()` | `bool` | 快速检查是否有待处理的变更 |

### fanout(扇出)

最重要也最复杂的组合算子。签名:

```rust
fn fanout<R: TriQueryLike<Value = Self::Key>>(self, other: R)
    -> DualQuery<ChainQuery<R::View, Self::View>, Arc<FastHashMap<R::Key, ValueChange<Self::Value>>>>
```

输入:
- `self`:以 A 为键的上游数据(X 值的视图 + 增量)
- `other`:A↔B 关系的 `TriQuery`(外键视图 + 增量 + 反向多索引)

输出:以 B 为键的数据(视图 = 通过外键链式查找,增量 = 物化的增量变更)。

增量计算分两个阶段进行:
- **关系变更**:当外键映射(B→A)改变时,查找新的/旧的 X 值
- **上游值变更**:当 X 改变时,通过反向关系扇出到受影响的 B 键;与阶段一结果相互抵消的条目会被移除

### 锁辅助

| 类型 | 说明 |
|------|-------------|
| `LockReadGuardHolder<T>` | 实现 `Query` 与 `MultiQuery` 的读锁守卫 |
| `LockWriteGuardHolder<T>` | 写锁守卫(可降级为读锁) |
| `MutexGuardHolder<T>` | 带 Deref/DerefMut 的互斥锁守卫 |

### 变更辅助

| 类型 / 函数 | 说明 |
|-----------------|-------------|
| `QueryLikeMutateTarget<K, V>` | 可变的键值存储 trait |
| `QueryMutationCollector<D, T>` | 包装目标 + 增量存储;变更时自动记录 |

### 反向关系簿记

| 函数 | 说明 |
|----------|-------------|
| `bookkeeping_hash_relation(mapping, changes)` | 从增量维护 `FastHashMap<V, FastHashSet<K>>` |
| `bookkeeping_dense_index_relation(mapping, changes)` | 同上,但用于 `DenseIndexMapping`(小列表 → 哈希表自适应) |

### 校验

| 函数 | 说明 |
|----------|-------------|
| `validate_query_consistency(q)` | 校验 iter_key_value/access/has_item_hint 的一致性 |
| `validate_multi_query_consistency(q)` | 校验 iter_keys/access_multi 的一致性 |

## 关键 trait(用于自定义实现)

| trait | 实现位置 | 用途 |
|-------|---------------|---------|
| `CKey` | 任何 Eq+Hash+CValue 类型 | 合法键类型的标记 |
| `CValue` | 任何 Clone+Send+Sync+Debug+PartialEq+'static | 合法值类型的标记 |
| `Query` | 你的容器 | 键 → 值只读访问 |
| `MultiQuery` | 你的容器 | 键 → 值集合只读访问 |
| `DualQueryLike` | 很少需要(2 个内置实现) | 视图 + 增量双查询抽象 |
| `TriQueryLike` | 很少需要(1 个内置实现) | 双查询 + 反向多索引 |
| `DataChanges` | 批量变更容器 | 分离的移除/插入迭代 |
| `IteratorProvider` | `[T; N]`、`Vec<T>` | 支持对集合进行 `Select`/`SelectChanges` |
| `QueryLikeMutateTarget` | 可变存储 | 支持 `QueryMutationCollector` 追踪 |
| `DynQuery` / `DynValueRefQuery` | 通过全覆盖实现自动实现 | Query 的动态派发 |
| `DynMultiQuery` | 通过全覆盖实现自动实现 | MultiQuery 的动态派发 |
