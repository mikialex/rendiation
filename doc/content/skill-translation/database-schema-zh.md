---
name: database-schema
description: >
  rendiation 类型安全关系数据库层(utility/database)的参考文档。
  涵盖通过 declare_entity!/declare_component! 定义表(实体类型)与列(组件)、
  表之间的显式外键、向全局数据库注册模式(schema)、通过 TableWriter/
  TableReader/ComponentReadView 进行 CRUD、查询模式、存储后端(线性 vs 稀疏)
  以及事件/钩子系统。
  在定义新的实体类型、添加组件、连接外键或直接与数据库层交互时使用。
metadata:
  version: "2.0"
  updated: "2026-05-17"
---

`utility/database` crate 提供了一个**类型安全的关系数据库**——而不是传统的 ECS(实体组件系统)。与 ECS 的关键区别:

- **多种实体类型**——与 ECS 只有单一实体类型不同,该数据库允许定义多种实体类型(类比 SQL 中的表)。
- **每张表的列固定**——每个组件是特定实体类型上的一个静态类型列。组件不能动态挂载/摘除;表中的每一行都有相同的列集合(空缺由默认值填充)。
- **显式外键**——实体间的关系通过 `declare_foreign_key!` 建模,而非通过组件查询。这与 SQL 中跨表关联行的外键类似。

关键文件:

| 文件 | 用途 |
| ------ | --------- |
| [utility/database/src/semantic.rs](../../../../../rendiation/utility/database/src/semantic.rs) | 模式定义宏与 trait |
| [utility/database/src/global.rs](../../../../../rendiation/utility/database/src/global.rs) | 全局单例 `Database` 的建立与访问 |
| [utility/database/src/kernel/](../../../../../rendiation/utility/database/src/kernel/) | 核心数据模型(列存储、句柄、写入器、读取器、查询) |
| [utility/database/src/storage/](../../../../../rendiation/utility/database/src/storage/) | 存储后端(线性 Vec 支撑、稀疏 HashMap 支撑) |
| [utility/database/src/hook/](../../../../../rendiation/utility/database/src/hook/) | 响应式钩子、变更通道、增量通道、引用计数 |

通过以下方式导入全部内容:

```rust
use database::*;
```

## 核心概念

### 实体类型 = 表

**实体类型**(`EntitySemantic`)定义一张表。每一行由一个 `EntityHandle<E>` 标识,并具有固定的列集合。与关系数据库中可以有多张表一样,你可以定义多种实体类型。

```rust
declare_entity!(SceneNodeEntity);   // a table of scene nodes
declare_entity!(SceneModelEntity);  // a table of scene models
declare_entity!(PbrSGMaterialEntity); // a table of PBR SG materials
```

### 组件 = 列

**组件**(`ComponentSemantic`)定义特定表中的一个列。它将数据类型 `Data` 与实体类型 `Entity` 关联。该表中的每一行都存储一个该类型的值。列是固定的——模式(schema)注册后,不能动态地为行添加或移除列。

```rust
declare_component!(SceneNodeLocalMatrixComponent, SceneNodeEntity, Mat4<f64>);
declare_component!(SceneNodeVisibleComponent, SceneNodeEntity, bool);
```

创建行时若未显式设置某列,该列将获得默认值(`Data::default()` 或自定义覆盖值)。

### 外键 = 跨表关系

**外键**是一种特殊的列,其数据类型固定为 `Option<RawEntityHandle>`,指向从某张表的一行到另一张表的一行。这显式地建模了实体类型之间的关系。

```rust
declare_foreign_key!(SceneModelRefNode, SceneModelEntity, SceneNodeEntity);
// SceneModelEntity.SceneModelRefNode → SceneNodeEntity
```

注意:内核不强制引用完整性——这是应用层的责任。

### 存储模型

每个列都有自己的物理存储。有两种后端:

- **线性(默认,`DBLinearStorage<T>`)** — 基于 Vec,密集存储。每一行占据一个槽位;被删除的行会留下空洞。注册组件时使用 `declare_component::<S>()` 方法。
- **稀疏(`DBSparseStorage<T>`)** — 基于 HashMap,只存储被写入过的行。适合很少填充的列,注册组件时使用 `declare_sparse_component::<S>()` 方法。

注意:存储后端的选择发生在向全局数据库注册组件时(见下文"注册"一节),而非在 `declare_component!` 声明宏中。

### 锁机制

- `TableWriter<E>` — 构造时获取表 `E` 的**所有**列上的写锁,析构时释放。同一时刻每张表只有一个写入者。
- `TableReader<E>` — 获取表 `E` 的**所有**列上的读锁。允许多个并发读取者。
- `ComponentReadView<C>` — 单个列上的读锁。
- `ComponentWriteView<C>` — 单个列上的写锁。

## 模式定义

### declare_entity! — 定义表

```rust
declare_entity!(MyEntity);
// Expands to:
//   pub struct MyEntity;
//   impl EntitySemantic for MyEntity {}
```

`EntitySemantic` 提供:

- `entity_id() -> EntityId` — 默认为 `TypeId::of::<Self>()`
- `unique_name() -> &'static str` — 默认为 `type_name::<Self>()`(必须稳定以便序列化)

### declare_component! — 定义列

```rust
declare_component!(CompName, EntityType, DataType);
// Optional custom default:
declare_component!(CompName, EntityType, DataType, DataType::custom_default());
```

创建实现以下语义的标记结构体:

- `EntityAssociateSemantic` — 将该列绑定到特定表
- `ComponentSemantic` — 设置 `Data = DataType`

**列数据类型要求**:`DataType` 必须实现 `DataBaseDataType`,该 trait 为满足 `CValue + Default + Facet + Serialize + Deserialize` 的任何类型提供了全覆盖实现(blanket impl)。实践中:

```rust
#[derive(Clone, Default, Facet, Serialize, Deserialize)]
struct MyColumnData {
    value: f32,
}
```

### declare_foreign_key! — 定义关系

```rust
declare_foreign_key!(FkName, OwnerEntity, ReferencedEntity);
```

创建一个 `Data` 固定为 `Option<RawEntityHandle>` 的列,实现 `ForeignKeySemantic`,其中 `type ForeignEntity = ReferencedEntity`。

### declare_entity_associated

```rust
declare_entity_associated!(TypeName, EntityType);
```

只实现 `EntityAssociateSemantic`,不实现 `ComponentSemantic`。用于需要绑定到表但不存储列数据的辅助标记类型。

## 注册

声明完模式(schema)后,将其注册到全局数据库。顺序很重要:先声明实体,再声明它的列与外键。

```rust
global_database()
    .declare_entity::<MyEntity>()
    .declare_component::<MyColumn>()
    .declare_component::<AnotherColumn>()
    .declare_foreign_key::<MyForeignKey>();
```

对于较大的子系统,通常会在初始化时通过 `register_xxx_data_model()` 函数注册所有表与列(参见 [scene/core/src/lib.rs](../../../../../rendiation/scene/core/src/lib.rs#L43) 中的 `register_scene_core_data_model()`)。

## CRUD 操作

### 创建行

```rust
let writer = global_entity_of::<MyEntity>().entity_writer();

let handle: EntityHandle<MyEntity> = writer.new_entity(|init| {
    init.write::<MyColumn>(&MyColumnData { value: 1.0 })
        .write::<AnotherColumn>(&default_value)
});
```

- `new_entity(init)` — 插入新行。`init` 闭包接收一个 `EntityInitWriteView` 用于设置初始列值。未被显式写入的列会得到默认值。
- `clone_entity(source)` — 深拷贝源行的所有列数据,返回带新 ID 的新句柄。
- `delete_entity(handle)` — 移除一行。内核不强制引用完整性。

### 写入列

```rust
// Via TableWriter (locks all columns of the table)
let mut writer = global_entity_of::<MyEntity>().entity_writer();
writer.write::<MyColumn>(handle, new_value);
writer.write_foreign_key::<MyFk>(handle, Some(other_handle));
writer.mutate_component_data::<MyColumn>(handle, |data| { data.value += 1.0; });

// Via ComponentWriteView (locks a single column)
let mut view = write_global_db_component::<MyColumn>();
view.write(handle, new_value);
```

### 读取列

```rust
// Via TableReader (locks all columns of the table)
let reader = global_entity_of::<MyEntity>().entity_reader();
let val: &MyColumnData = reader.get::<MyColumn>(handle);
let opt: Option<&AnotherData> = reader.try_get::<AnotherColumn>(handle);
let fk: Option<EntityHandle<RefEntity>> = reader.read_foreign_key::<MyFk>(handle);

// Via ComponentReadView (locks a single column)
let view = read_global_db_component::<MyColumn>();
let val: Option<&MyColumnData> = view.get(handle);

// Via foreign key read view (resolves FK → typed handle)
let fk_view = read_global_db_foreign_key::<MyFk>();
let fk: Option<EntityHandle<RefEntity>> = fk_view.get(handle);
```

## 查询模式

### 扫描一列

```rust
let view: IterableComponentReadViewChecked<MyData> = get_db_view::<MyColumn>();
for (handle, value) in view.iter_key_value() {
    // handle: RawEntityHandle, value: MyData
}
```

### 扫描表中的所有行

```rust
let set_view = get_db_set_view::<MyEntity>();
for (handle, ()) in set_view.iter_key_value() { ... }
```

### 检查行在列中是否有非默认值

```rust
let view = read_global_db_component::<MyColumn>();
if let Some(value) = view.get(handle) { ... }
```

## 响应式钩子

`utility/database/src/hook/` 中的钩子系统为增量更新提供响应式查询:

- `use_changes::<C>()` — 获取列 C 的变更流
- `use_dual_query::<C>()` — 一次性获取当前状态 + 增量流
- `use_query_set::<E>()` — 获取表 E 的行插入/删除事件
- `use_db_rev_ref::<C>()` — 获取反向引用映射(外键的逆,即"哪些行指向这一行?")

场景层使用这些钩子来增量推导世界变换、传播可见性并计算包围体。

## 模式参考

```rust
// Define schema (table + columns + foreign key)
declare_entity!(MyTable);
declare_component!(ColumnA, MyTable, (f32, f32));
declare_component!(ColumnB, MyTable, f32);
declare_foreign_key!(FkRef, MyTable, OtherTable);

// Register with global database
global_database()
    .declare_entity::<MyTable>()
    .declare_component::<ColumnA>()
    .declare_component::<ColumnB>()
    .declare_foreign_key::<FkRef>();

// Insert a row
let handle = global_entity_of::<MyTable>()
    .entity_writer()
    .new_entity(|w| w.write::<ColumnB>(&1.0));

// Read a column
let reader = global_entity_of::<MyTable>().entity_reader();
let val = reader.read::<ColumnB>(handle);

// Update a column
let mut writer = global_entity_of::<MyTable>().entity_writer();
writer.write::<ColumnB>(handle, 2.0);

// Scan all values in a column
let view = get_db_view::<ColumnB>();
for (handle, value) in view.iter_key_value() {
    println!("row {:?} has value {}", handle, value);
}
```
