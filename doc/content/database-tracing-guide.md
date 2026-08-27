# Rendiation 数据库追踪（database-tracing）指南（utility/database-tracing）

本文梳理 [utility/database-tracing](../../utility/database-tracing) 的数据库追踪抽象体系：把关系数据库的全部变更记录为紧凑的二进制 trace 文件，并在之后加载、回放或转换成可读文本。它解决的问题是"应用行为发生了但无法复现"——有了 trace，整个场景的构造过程可以逐帧重放进任意时刻的数据库实例里。

## 前置阅读

database-tracing 是 [utility/database](../../utility/database) 之上的观察者层，它订阅数据库的事件源并把变更序列化。建议先了解：

| 文档 | 内容 |
| --- | --- |
| [skill-translation/database-schema-zh.md](skill-translation/database-schema-zh.md) | 类型安全关系数据库：实体/组件/外键、注册、CRUD 与钩子系统 |
| [skill-translation/query-system-zh.md](skill-translation/query-system-zh.md) | 增量查询系统，`ValueChange` 增量模型的来源 |
| [query-hook-guide.md](query-hook-guide.md) | 两阶段执行模型（spawn/resolve），viewer 侧回放面板依赖的 hook 机制 |
| [hooks-guide.md](hooks-guide.md) | hook 运行时与状态管理，`use_enable_trace_io` 的宿主 |

## 模式概览

一套完整的使用流是：在应用启动早期（数据库为空时）调用 `start_tracing`，把 `TraceWriter`（如 `FileTraceWriter`）挂到数据库的所有变更事件上；应用运行期间，数据库的每次实体创建/删除、每个组件写值都会被序列化成一个 `TracingMessage`，应用自身也可以插入自定义事件（帧标记、C API 调用记录）。之后：

- **转文本**：用 `trace_to_text` 或 `ReplayTypeRegistry::convert_to_text` 把二进制 trace 变成可读的行文本，用于事后分析。
- **回放**：用 `ReplayTypeRegistry::load` 或 `load_replay` 把 trace 读成 `ReplayState`，再用 `step_forward` 逐帧应用到一个活的数据库实例上，完整重建当时的数据状态。

核心设计选择：

- 数据库写入路径（`data_watchers` 等事件源）与文件 IO 完全解耦：`TraceWriter` 只需收消息，`FileTraceWriter` 用无界 channel + 后台线程写文件，不阻塞数据库热路径。
- 文件自描述：文件头写入一个名称表（实体名与组件名），记录里只存紧凑的 u32 名称索引，不依赖数据库内部的类型 ID 或注册顺序。
- 事件流里有"回放目标"标记（如 viewer 的每帧 `Render` 事件）：回放器可以按这些标记切分帧边界，逐帧前进。
- 回放时句柄要重映射：trace 里的 `RawEntityHandle` 属于录制时的分配器，重放数据库有自己的分配器，两者必须通过映射表对应起来。

## 核心概念

| 概念 | 定义 | 说明 |
| --- | --- | --- |
| `start_tracing` | [utility/database-tracing/src/lib.rs](../../utility/database-tracing/src/lib.rs) | 追踪入口：构建名称表、写文件头、订阅全部数据库变更事件并转发给 writer |
| `build_name_table` | 同上 | 遍历已注册表，把实体名与组件名编进同一张名称表，分配 u32 索引 |
| `NameTable` | [utility/database-tracing/src/message.rs](../../utility/database-tracing/src/message.rs) | 名称表结构：`names` 字符串表 + 实体/组件到索引的两个映射 |
| `TracingMessage<T>` | 同上 | 顶层记录枚举：`Event(T)` 用户自定义事件或 `DatabaseMutation(DatabaseTracingMessage)` 数据库变更 |
| `DatabaseTracingMessage` | 同上 | 数据库变更记录：`EntityCreated` / `EntityDeleted` / `EntityFieldSet`，均携带名称索引与句柄 |
| `EntityFieldData` | 同上 | 组件值载荷：普通组件为 msgpack 序列化字节（`Pod`），外键直接存 `Option<RawEntityHandle>` 便于重映射 |
| `TraceIO` | 同上 | 记录的自描述 IO trait：`write_len` / `write` / `read` 三方契约，决定二进制格式 |
| `TraceWriter<T>` | [utility/database-tracing/src/writer.rs](../../utility/database-tracing/src/writer.rs) | 消息接收端 trait：`write_header` 写协议头，`write_message` 收记录；要求 `Clone` 以分发给多个订阅回调 |
| `FileTraceWriter<T>` | 同上 | 文件实现：无界 channel + 后台线程串行写盘，构造时截断旧文件 |
| `TraceReplayTarget` | [utility/database-tracing/src/replay.rs](../../utility/database-tracing/src/replay.rs) | 事件类型 trait：`type_discriminant` 唯一标识（写入文件头），`is_replay_target` 标记回放边界 |
| `ReplayTypeRegistry` | 同上 | 按 discriminant 注册多种事件类型的运行时分发器，一次加载任意兼容 trace 文件 |
| `ReplayState` | 同上 | 回放状态：已解析记录、当前位置、名称表、原始句柄到活句柄的映射 |
| `ParsedRecord` / `RecordKind` | 同上 | 单条记录的解析视图：索引、摘要文本、类别、是否回放目标 |
| `step_forward` 等 | 同上 | 回放驱动器：`step_forward` 前进到下一个目标事件，`step_forward_single` 单步，`restart_and_run_to` 复位重放 |
| `trace_to_text` | [utility/database-tracing/src/lib.rs](../../utility/database-tracing/src/lib.rs) | 通用转文本入口，可选传入活的 `Database` 以解码组件值 |
| `ViewerTracingEvent` | [application/viewer/src/db_tracing.rs](../../application/viewer/src/db_tracing.rs) | viewer 的自定义事件：每帧一个 `Render` 事件，即回放帧边界 |
| `RendiationCxAPITraceEvent` | [application/viewer-content-api-trace-info/src/lib.rs](../../application/viewer-content-api-trace-info/src/lib.rs) | C API 的自定义事件：每次 API 调用一个事件，msgpack 序列化 |

## 为什么需要这套分层

看数据流就能理解各组件的位置。数据库内核的每次变更都会通过事件源广播（`data_watchers` 广播组件写值、`entity_watchers` 广播实体创建/删除，见 [utility/database/src/kernel/component.rs](../../utility/database/src/kernel/component.rs) 与 [kernel/table.rs](../../utility/database/src/kernel/table.rs)），database-tracing 只是这些事件源的另一个订阅者：

```text
数据库内核变更
  data_watchers（组件写值）/ entity_watchers（实体增删）
  └─ start_tracing 订阅（每个表、每个组件各挂一个回调）
       ├─ 组件数据 → serialize_into_buffer（msgpack 字节）
       ├─ 外键列 → Option<RawEntityHandle> 直接记录
       └─ 实体增删 → 名称索引 + 句柄
  + 应用层事件（帧 Render、C API 调用）
  → TracingMessage 流
  → TraceWriter::write_message（不阻塞调用方）
  → FileTraceWriter：channel → 后台线程 → trace.bin
       ├─ ReplayTypeRegistry::load / load_replay → ReplayState
       │    └─ step_forward（按目标事件切帧）→ 写入活的 Database
       └─ convert_to_text / trace_to_text → trace.txt
```

分层动机：

- **订阅层与格式层分离**（lib.rs 与 message.rs）：`start_tracing` 只负责把事件源转成 `TracingMessage`，格式编码全部收敛在 `TraceIO` 实现里，替换文件格式不影响订阅逻辑。
- **写入与记录分离**（writer.rs）：数据库写值发生在锁内热路径，`TraceWriter` 接口让写入方只投递消息；`FileTraceWriter` 把阻塞 IO 挪到后台线程。自定义 writer 可以改成网络发送或其他传输。
- **事件类型参数化**：`TracingMessage<T>` 的 `T` 承载应用自定义事件，`TraceReplayTarget` 提供判别符与帧边界——数据库变更记录与业务事件共用同一条记录流，回放器据此知道"这一步走到哪一帧"。
- **文件自描述**：名称表写进文件头，回放/转文本不依赖录制时的注册顺序与内部 ID，只要名称还在就能对得上。
- **句柄重映射独立于回放应用**：`ReplayState.handle_map` 把录制句柄映射到当前数据库的活句柄，回放同一份 trace 到不同数据库实例都能工作。

## 订阅数据库变更：start_tracing

[lib.rs](../../utility/database-tracing/src/lib.rs) 的 `start_tracing` 分四步：

- 用 `build_name_table` 遍历 `database.tables`，先给每个实体名分配索引，再遍历每个表的组件名继续分配——实体名与组件名共享同一个索引空间（`EntityCreated` 里的名称索引指实体名，`EntityFieldSet` 里的指组件名）。
- `writer.write_header(&name_table, T::type_discriminant())` 先写文件头，之后才是记录流。
- 对每个表断言 `living_entity_count() == 0`：追踪必须在任何数据写入之前开启，否则那些已存在实体的历史无法被记录。`entity_meta_watcher` 与 `component_define_watchers` 的订阅回调直接 `unreachable!`——运行期新注册表/组件不受支持。
- 每个表挂两个回调：`entity_watchers()` 处理 `EntityChange::NewEntityStartCreate` 与 `DeleteEntity`（`NewEntityCreated` 忽略，创建起点已记录）；每个组件挂 `data_watchers` 回调，处理 `ValueChange::Delta`。

组件数据回调（[lib.rs](../../utility/database-tracing/src/lib.rs) 第 101 行起）的关键在类型分派：

```rust
let field_data = if c_is_fk {
  let fk = (data_ptr as *const Option<RawEntityHandle>).read();
  EntityFieldData::ForeignKey(fk)          // 外键：直接记录原始句柄
} else {
  let new = &*dyn_ptr as &dyn DynDataBaseDataType;
  let buffer = new.serialize_into_buffer(); // 普通组件：msgpack 序列化
  EntityFieldData::Pod(buffer.to_vec())
};
```

变更消息携带 `(DataPtr, *const dyn DynDataBaseDataType)`（[kernel/component.rs](../../utility/database/src/kernel/component.rs) 的 `ChangePtr`），指针只在回调内有效，所以序列化必须在回调内完成——代码里也留有"把序列化移到 writer 线程"的 TODO。`ValueChange::Remove` 被忽略（实体删除由 `EntityDeleted` 事件覆盖）。外键直接存 `RawEntityHandle` 而不序列化，是为了回放时能直接做句柄重映射。

订阅回调返回 `false`：事件源的 `on` 回调返回值表示是否移除自身（见 [utility/event-source/src/source.rs](../../utility/event-source/src/source.rs)），追踪订阅持续整个生命周期。

## 记录格式与 TraceIO

二进制格式全部收敛在 [message.rs](../../utility/database-tracing/src/message.rs)：

- 文件头：4 字节魔数 `RTRC` + 版本号（当前 1）+ 头长（20）+ `type_discriminant`（u32）+ 名称数量 + 每条名称（u16 长度前缀 + UTF-8 字节），由 `write_trace_file_header` / `read_trace_file_header` 读写。
- 每条记录：u32 记录长度 + 载荷。载荷首个字节是 tag：`0x00` Event、`0x01` EntityCreated、`0x02` EntityDeleted、`0x03` EntityFieldSet。
- `EntityCreated` / `EntityDeleted` 载荷固定 16 字节：名称索引（u32）+ 句柄（u32 分配索引 + u64 代数）。
- `EntityFieldSet` 载荷：名称索引 + 句柄 + 1 字节是否外键；普通组件再跟 u32 数据长度 + msgpack 字节；外键再跟 1 字节是否有值 + 句柄（无值则全零占位）。

`TraceIO` 是记录类型的自描述契约（[message.rs](../../utility/database-tracing/src/message.rs)）：

```rust
pub trait TraceIO: Debug {
  fn write_len(&self) -> usize;                    // 必须在 write 之前可调用
  fn write(&self, w: &mut impl Write) -> std::io::Result<usize>;
  fn read(source: &mut dyn Read) -> std::io::Result<Self> where Self: Sized;
}
```

`()` 的空实现让无自定义事件的场景直接使用；`TracingMessage<T>` 在 `T: TraceIO` 时实现 `TraceIO`，帧的定长计数由 read 端（读 u32 长度后读入缓冲）完成。`DatabaseTracingMessage` 的 `read` 用 `RawEntityHandle::create_only_for_testing_with_gen` 重建句柄——句柄是无校验的裸数值对，重放合法性由回放逻辑负责。

## 写入侧：TraceWriter 与 FileTraceWriter

[writer.rs](../../utility/database-tracing/src/writer.rs) 只做一件事：定义消息接收端。

`TraceWriter<T>` 要求 `Clone`：`start_tracing` 会把同一个 writer clone 进每个订阅回调，闭包因此可以 `move`。`write_header` 与 `write_message` 的调用时机有约定——header 恰好调用一次，在所有记录之前。

`FileTraceWriter` 的实现：构造时打开文件并截断（`truncate(true)` + 显式 `set_len(0)` 双保险），然后起一个后台线程：

```rust
let (sender, mut receiver) = futures::channel::mpsc::unbounded::<T>();
std::thread::spawn(move || {
  while let Some(data) = futures::executor::block_on(receiver.next()) {
    data.write(&mut *file_clone.lock()).unwrap();
  }
  file_clone.lock().flush().unwrap();
});
```

`write_message` 只是 `unbounded_send`——调用方（数据库写路径）零阻塞；文件写入与落盘是后台线程的串行工作。写失败会直接 unwrap panic，因为 trace 损坏比崩溃更糟糕。头文件经 `write_header` 同步写入（持锁），保证在第一条记录前落盘。

## 自定义事件与回放边界

`TracingMessage::Event(T)` 是应用插入业务标记的唯一通道，两个现成实现展示了两种风格：

- `ViewerTracingEvent`（[application/viewer/src/db_tracing.rs](../../application/viewer/src/db_tracing.rs)）：单个 `Render` 变体，手工二进制编码（1 字节 tag），`is_replay_target` 恒为 true。viewer 渲染循环每帧调用一次（[application/viewer/src/viewer/mod.rs](../../application/viewer/src/viewer/mod.rs) 第 520 行），于是每帧边界在 trace 里都有一个标记，回放可以逐帧前进。
- `RendiationCxAPITraceEvent`（[application/viewer-content-api-trace-info/src/lib.rs](../../application/viewer-content-api-trace-info/src/lib.rs)）：枚举覆盖 C API 的全部可观察调用（创建/调整 surface、拾取、派生查询等），用 `rmp_serde` 整体 msgpack 序列化；`is_replay_target` 恒为 true。它的 discriminant 是 11，与 viewer 的 10 区分开。

`TraceReplayTarget` 的两个方法各司其职：`type_discriminant` 写进文件头，加载时校验/分发；`is_replay_target` 只影响回放切帧，不进入二进制流。

## 回放：ReplayTypeRegistry 与 ReplayState

[replay.rs](../../utility/database-tracing/src/replay.rs) 提供两个加载入口：

- `load_replay::<T>`：编译期指定事件类型，校验文件头的 discriminant 与 `T::type_discriminant()` 一致，不一致报错拒绝。
- `ReplayTypeRegistry`：运行时分发。`register::<T>()` 把类型擦除成两个函数指针（`RecordLoader` 与 `TextConverter`）存入以 discriminant 为键的表；`load` 先读文件头取 discriminant，再按注册表找到对应的解析函数。viewer 的 `replay_registry()`（[application/viewer/src/viewer/feature/trace_io.rs](../../application/viewer/src/viewer/feature/trace_io.rs)）一次注册两个事件类型，因此一个 registry 可以加载两种 trace 文件。

`ReplayState` 是回放的完整状态：`records`（解析后的记录列表）、`position`（当前位置）、`names`（从文件头恢复的名称表）、`handle_map`（`EntityId → FastHashMap<RawEntityHandle, RawEntityHandle>`）。

`apply_single` 把单条 `RecordKind` 应用到数据库（[replay.rs](../../utility/database-tracing/src/replay.rs)）：

- `EntityCreated`：按名称解析出实体类型（`resolve_entity_id` 查 `db.name_mapping`），用 `entity_writer_untyped_dyn` 建实体，并把 `原始句柄 → 活句柄` 记入该实体类型的映射表。
- `EntityDeleted`：查映射表找到活句柄，删除并移除映射。
- `EntityFieldSet`：按组件名解析出组件，通过映射表找到所属实体的活句柄；`Pod` 数据经 `write_by_small_serialize_data` 反序列化写入，外键值则在 `handle_map` 里二次映射成当前数据库的活句柄后写入（外键因此必须存原始句柄而不是序列化）。

句柄映射按实体类型分开是有原因的：每个实体类型有独立的分配器（注释明确指出"同一个 `RawEntityHandle` 值可能出现在不同实体类型中"），混在一起会错配。

驱动器三个函数：

- `step_forward`：循环应用记录，直到应用完一个 `is_replay_target` 记录为止（含）。一次调用 = 一帧。
- `step_forward_single`：只应用当前记录，用于逐条检查。
- `restart_and_run_to`：清空句柄映射、位置归零，重放到指定索引——viewer 面板点击记录行跳转就是用它。

## 转文本：trace_to_text

[lib.rs](../../utility/database-tracing/src/lib.rs) 的 `trace_to_text` 把整个文件转成逐行文本：读文件头恢复名称表，循环 `TracingMessage::<T>::read`，用 `format_message` 格式化（`[EntityCreated] entity="..." handle=(idx, g:gen)` 之类）。

`EntityFieldSet` 的值渲染是可选的深度解码：若传入活的 `db`，`format_component_value` 会按组件名查 `db.name_mapping` 找到组件，再取 `binary_to_debug_string`（注册时由 `create_binary_to_debug_string` 生成的"反序列化 + Debug 格式化"函数指针，见 [utility/database/src/storage/mod.rs](../../utility/database/src/storage/mod.rs)）把 msgpack 字节还原成 `{:?}` 文本；超过 `max_data_debug_len` 的大载荷只显示 `data_len=N`，schema 对不上（如组件已改名）也会降级显示。不传 `db` 时所有组件值都只显示长度——这使纯离线分析也能看 trace 结构。

## 下游使用

| 使用点 | 位置 | 用途 |
| --- | --- | --- |
| viewer 追踪启用 | [application/viewer/src/main.rs](../../application/viewer/src/main.rs) | 配置 `enable_tracing_and_tracing_write_path` 非空时创建 `FileTraceWriter` 并调 `start_tracing`，渲染循环每帧写入 `Event(Render)` |
| viewer 回放/转文本面板 | [application/viewer/src/viewer/feature/trace_io.rs](../../application/viewer/src/viewer/feature/trace_io.rs) | "Trace IO" 窗口：convert-trace 终端命令把 .bin 转 .txt；加载 trace 后逐帧 `step_forward` 回放进 live 数据库，记录表支持点击跳转（`restart_and_run_to`） |
| C API 追踪 | [application/viewer-content-api/src/trace.rs](../../application/viewer-content-api/src/trace.rs) | `setup_tracing(trace_write_path)` 从 C 传入路径开启追踪，`APITraceEventSender` 供各 API 埋点 emit 事件 |
| C API 事件埋点 | [application/viewer-content-api/src/viewer_api.rs](../../application/viewer-content-api/src/viewer_api.rs)、[bbox.rs](../../application/viewer-content-api/src/bbox.rs) | 拾取、包围盒等 API 调用处 `expect_tracing_event_emitter().emit(&RendiationCxAPITraceEvent::...)` |

## 使用模板

### 模板一：启用追踪并记录帧边界

```rust
// 应用初始化早期（数据库注册完成、未写任何数据时）
let writer = FileTraceWriter::<TracingMessage<ViewerTracingEvent>>::new(trace_write_path);
let notifier = start_tracing(&global_database(), writer);
// 每帧
notifier.write_message(TracingMessage::Event(ViewerTracingEvent::Render));
```

### 模板二：自定义事件类型

实现三个 trait 即可接入（以 viewer 的 `ViewerTracingEvent` 为最小范例）：

```rust
#[derive(Debug)]
pub enum MyTraceEvent { Frame, /* ... */ }

impl database_tracing::TraceReplayTarget for MyTraceEvent {
  fn type_discriminant() -> u32 { 42 }          // 与已有类型区分
  fn is_replay_target(&self) -> bool { true }   // 帧边界
}

impl database_tracing::TraceIO for MyTraceEvent {
  fn write_len(&self) -> usize { 1 }
  fn write(&self, w: &mut impl std::io::Write) -> std::io::Result<usize> { /* 编码 */ }
  fn read(source: &mut dyn std::io::Read) -> std::io::Result<Self> { /* 解码 */ }
}
```

事件体较大时可以直接用 `rmp_serde` 序列化整个结构体（参考 `RendiationCxAPITraceEvent`）。

### 模板三：回放 trace

```rust
// 单类型场景：编译期校验
let mut state = database_tracing::load_replay::<MyTraceEvent>(path)?;
step_forward(&mut state, &global_database()); // 前进一帧

// 多类型场景：注册表分发
let mut registry = ReplayTypeRegistry::new();
registry.register::<MyTraceEvent>();
registry.register::<OtherEvent>();
let loaded = registry.load(path)?;
restart_and_run_to(&mut loaded.state, &global_database(), target_index);
```

### 模板四：转文本调试

```rust
// 传入 live 数据库可解码组件值；传 None 则只看记录结构
let mut out = std::fs::File::create("trace.txt")?;
trace_to_text::<MyTraceEvent>("trace.bin", &mut out, Some(&global_database()), 1024)?;
```

## 延伸阅读

- 事件源模型（`on` / `emit` / 移除 token）：[utility/event-source/src/source.rs](../../utility/event-source/src/source.rs)
- 组件变更广播与未类型化写入视图（`data_watchers`、`write_untyped`、`write_by_small_serialize_data`）：[utility/database/src/kernel/component.rs](../../utility/database/src/kernel/component.rs)
- 序列化契约与调试字符串（`DataBaseDataType`、`create_binary_to_debug_string`、`DatabaseSerializedFieldBufferOrForeignKey`）：[utility/database/src/storage/mod.rs](../../utility/database/src/storage/mod.rs)
- 表级事件源与名称映射（`entity_watchers`、`component_define_watchers`、`name_mapping`）：[utility/database/src/kernel/table.rs](../../utility/database/src/kernel/table.rs)、[kernel/entry.rs](../../utility/database/src/kernel/entry.rs)
- `ValueChange` 增量模型定义：[utility/query/src/delta_query/delta.rs](../../utility/query/src/delta_query/delta.rs)
