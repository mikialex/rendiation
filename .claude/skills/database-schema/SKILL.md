---
name: database-schema
description: >
  Reference for the rendiation type-safe relational database layer (utility/database).
  Covers defining tables (entity types) and columns (components) with declare_entity!/
  declare_component!, explicit foreign keys between tables, registering schemas with
  the global database, CRUD via EntityWriter/EntityReader/ComponentReadView, query
  patterns, storage backends (linear vs sparse), and the event/hook system.
  Use when defining new entity types, adding components, wiring foreign keys, or
  interacting with the database layer directly.
metadata:
  version: "2.0"
  updated: "2026-05-17"
---

The `utility/database` crate provides a **type-safe relational database** — not a traditional ECS. Key differences from ECS:

- **Multiple entity types** — unlike ECS's single entity type, this database lets you define many entity types (analogous to tables in SQL).
- **Fixed columns per table** — each component is a statically-typed column on a specific entity type. Components are not dynamically attached/detached; every row in a table has the same set of columns (with default values filling gaps).
- **Explicit foreign keys** — relationships between entities are modeled via `declare_foreign_key!`, not through component queries. This is similar to SQL foreign keys linking rows across tables.

Key files:

| File | Purpose |
|------|---------|
| [utility/database/src/semantic.rs](utility/database/src/semantic.rs) | Schema-definition macros and traits |
| [utility/database/src/global.rs](utility/database/src/global.rs) | Global singleton `Database` setup and access |
| [utility/database/src/kernel/](utility/database/src/kernel/) | Core data model (column stores, handles, writers, readers, queries) |
| [utility/database/src/storage/](utility/database/src/storage/) | Storage backends (linear Vec-backed, sparse HashMap-backed) |
| [utility/database/src/hook/](utility/database/src/hook/) | Reactive hooks, change channels, delta channels, ref-counting |

Import everything with:

```rust
use database::*;
```

## Core concepts

### Entity types = tables

An **entity type** (`EntitySemantic`) defines a table. Each row is identified by an `EntityHandle<E>` and has a fixed set of columns. You can have many entity types, just as you can have many tables in a relational database.

```rust
declare_entity!(SceneNodeEntity);   // a table of scene nodes
declare_entity!(SceneModelEntity);  // a table of scene models
declare_entity!(PbrSGMaterialEntity); // a table of PBR SG materials
```

### Components = columns

A **component** (`ComponentSemantic`) defines a column in a specific table. It associates a data type `Data` with an entity type `Entity`. Every row in that table stores one value of that type. Columns are fixed — you cannot dynamically add or remove columns from a row after the schema is registered.

```rust
declare_component!(SceneNodeLocalMatrixComponent, SceneNodeEntity, Mat4<f64>);
declare_component!(SceneNodeVisibleComponent, SceneNodeEntity, bool);
```

If a column is not explicitly set when creating a row, it gets the default value (`Data::default()` or a custom override).

### Foreign keys = relationships across tables

A **foreign key** is a special column whose data type is fixed to `Option<RawEntityHandle>`, pointing from a row in one table to a row in another table. This explicitly models relationships between entity types.

```rust
declare_foreign_key!(SceneModelRefNode, SceneModelEntity, SceneNodeEntity);
// SceneModelEntity.SceneModelRefNode → SceneNodeEntity
```

Note: referential integrity is NOT enforced by the kernel — it's the application layer's responsibility.

### Storage model

Each column gets its own physical store. Two backends:

- **Linear** (default, `DBLinearStorage<T>`) — Vec-backed, dense. Every row occupies a slot; deleted rows leave holes. Use `declare_component!`.
- **Sparse** (`DBSparseStorage<T>`) — HashMap-backed, only stores rows that have been written. Use `declare_sparse_component` for columns that are rarely populated.

### Locking

- `EntityWriter<E>` — acquires write locks on ALL columns of table `E` on construction, releases on drop. One writer per table at a time.
- `EntityReader<E>` — acquires read locks on ALL columns of table `E`. Multiple concurrent readers allowed.
- `ComponentReadView<C>` — read lock on a single column.
- `ComponentWriteView<C>` — write lock on a single column.

## Schema definition

### declare_entity! — define a table

```rust
declare_entity!(MyEntity);
// Expands to:
//   pub struct MyEntity;
//   impl EntitySemantic for MyEntity {}
```

`EntitySemantic` provides:
- `entity_id() -> EntityId` — defaults to `TypeId::of::<Self>()`
- `unique_name() -> &'static str` — defaults to `type_name::<Self>()` (must be stable for serialization)

### declare_component! — define a column

```rust
declare_component!(CompName, EntityType, DataType);
// Optional custom default:
declare_component!(CompName, EntityType, DataType, DataType::custom_default());
```

Creates a marker struct implementing:
- `EntityAssociateSemantic` — binds this column to a specific table
- `ComponentSemantic` — sets `Data = DataType`

**Column data requirements**: `DataType` must implement `DataBaseDataType`, which has a blanket impl for any type satisfying `CValue + Default + Facet + Serialize + Deserialize`. In practice:

```rust
#[derive(Clone, Default, Facet, Serialize, Deserialize)]
struct MyColumnData {
    value: f32,
}
```

### declare_foreign_key! — define a relationship

```rust
declare_foreign_key!(FkName, OwnerEntity, ReferencedEntity);
```

Creates a column whose `Data` is fixed to `Option<RawEntityHandle>`, implementing `ForeignKeySemantic` with `type ForeignEntity = ReferencedEntity`.

### declare_entity_associated!

```rust
declare_entity_associated!(TypeName, EntityType);
```

Only implements `EntityAssociateSemantic`, no `ComponentSemantic`. Use for auxiliary marker types that need to be bound to a table but don't store column data.

## Registration

After declaring schemas, register them with the global database. The order matters: declare entity first, then its columns and foreign keys.

```rust
global_database()
    .declare_entity::<MyEntity>()
    .declare_component::<MyColumn>()
    .declare_component::<AnotherColumn>()
    .declare_foreign_key::<MyForeignKey>();
```

For larger subsystems, there's typically a `register_xxx_data_model()` function that registers all tables and columns at init time (see [scene/core/src/lib.rs](scene/core/src/lib.rs#L44): `register_scene_core_data_model()`).

## CRUD operations

### Creating rows

```rust
let writer = global_entity_of::<MyEntity>().entity_writer();

let handle: EntityHandle<MyEntity> = writer.new_entity(|init| {
    init.write::<MyColumn>(&MyColumnData { value: 1.0 })
        .write::<AnotherColumn>(&default_value)
});
```

- `new_entity(init)` — inserts a new row. The `init` closure receives an `EntityInitWriteView` for setting initial column values. Columns not explicitly written receive their default value.
- `clone_entity(source)` — deep-copies all column data from the source row, returns new handle with a new ID.
- `delete_entity(handle)` — removes a row. Reference integrity is NOT enforced by the kernel.

### Writing columns

```rust
// Via EntityWriter (locks all columns of the table)
let mut writer = global_entity_of::<MyEntity>().entity_writer();
writer.write::<MyColumn>(handle, new_value);
writer.write_foreign_key::<MyFk>(handle, Some(other_handle));
writer.mutate_component_data::<MyColumn>(handle, |data| { data.value += 1.0; });

// Via ComponentWriteView (locks a single column)
let mut view = write_global_db_component::<MyColumn>();
view.write(handle, new_value);
```

### Reading columns

```rust
// Via EntityReader (locks all columns of the table)
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

## Query patterns

### Scanning a column

```rust
let view: IterableComponentReadViewChecked<MyData> = get_db_view::<MyColumn>();
for (handle, value) in view.iter_key_value() {
    // handle: RawEntityHandle, value: MyData
}
```

### Scanning all rows of a table

```rust
let set_view = get_db_set_view::<MyEntity>();
for (handle, ()) in set_view.iter_key_value() { ... }
```

### Checking if a row has a non-default value in a column

```rust
let view = read_global_db_component::<MyColumn>();
if let Some(value) = view.get(handle) { ... }
```

## Reactive hooks

The hook system in `utility/database/src/hook/` provides reactive queries for incremental updates:

- `use_changes::<C>()` — get a change stream for column C
- `use_dual_query::<C>()` — get current state + delta stream in one
- `use_query_set::<E>()` — get row insertion/deletion events for table E
- `use_db_rev_ref::<C>()` — get reverse-reference mapping (inverse of foreign keys, i.e. "which rows point to this row?")

These are used by the scene layer to derive world transforms, propagate visibility, and compute bounding volumes incrementally.

## Pattern reference

```rust
// 1. Define schema (table + columns + foreign key)
declare_entity!(MyTable);
declare_component!(ColumnA, MyTable, (f32, f32));
declare_component!(ColumnB, MyTable, f32);
declare_foreign_key!(FkRef, MyTable, OtherTable);

// 2. Register with global database
global_database()
    .declare_entity::<MyTable>()
    .declare_component::<ColumnA>()
    .declare_component::<ColumnB>()
    .declare_foreign_key::<FkRef>();

// 3. Insert a row
let handle = global_entity_of::<MyTable>()
    .entity_writer()
    .new_entity(|w| w.write::<ColumnB>(&1.0));

// 4. Read a column
let reader = global_entity_of::<MyTable>().entity_reader();
let val = reader.read::<ColumnB>(handle);

// 5. Update a column
let mut writer = global_entity_of::<MyTable>().entity_writer();
writer.write::<ColumnB>(handle, 2.0);

// 6. Scan all values in a column
let view = get_db_view::<ColumnB>();
for (handle, value) in view.iter_key_value() {
    println!("row {:?} has value {}", handle, value);
}
```
