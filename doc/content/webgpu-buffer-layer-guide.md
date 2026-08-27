# Rendiation WebGPU 缓冲资源层指南（platform/graphics/webgpu/src/resource/buffer）

本文梳理 [platform/graphics/webgpu/src/resource/buffer](../../platform/graphics/webgpu/src/resource/buffer) 的缓冲资源层：这是整个渲染侧 GPU 缓冲的地基。`AbstractBuffer` / `AbstractStorageAllocator` 提供了"与具体平台资源解耦、可插拔分配策略"的缓冲抽象；`linear_buffer_array` 组合子体系（`with_direct_resize` / `with_vec_backup` / `with_default_grow_behavior` / `CustomGrowBehaviorMaintainer` 等）把"可增长、可重定位、带宿主备份、带默认值"等维护行为像乐高一样叠加在任意缓冲之上；`allocator` 子层（slab 与 range 分配器）则回答"实体在缓冲里的槽位怎么分"。

[webgpu-hook-utils-guide.md](webgpu-hook-utils-guide.md) 的稀疏更新存储缓冲、[batch-extractor-guide.md](batch-extractor-guide.md) 的 GPU id 池、ray-tracing 的 SBT（Shader Binding Table）都直接建立在本文的抽象之上，它们会内联引用本文的语义。

## 前置阅读

本文的核心概念依赖 shader 类型系统与两阶段 hook 运行时，建议先了解：

| 文档 | 内容 |
| --- | --- |
| [skill-translation/shader-edsl-binding-and-typed-container-zh.md](skill-translation/shader-edsl-binding-and-typed-container-zh.md) | 类型化 GPU 资源容器（`UniformBufferDataView` / `StorageBufferReadonlyDataView` 等）与着色器侧 `bind_by`、通道侧 `bind` 的双向绑定 |
| [skill-translation/shader-edsl-core-zh.md](skill-translation/shader-edsl-core-zh.md) | Std430 / Std140 内存布局标注、`ShaderPtr` 指针类型（本文的"类型描述" `MaybeUnsizedValueType` 与布局目标 `StructLayoutTarget` 的来源） |
| [webgpu-hook-utils-guide.md](webgpu-hook-utils-guide.md) | 下游之一：两阶段 GPU 维护、稀疏写、范围分配器的批分配封装（本层 API 的最大消费方） |
| [batch-extractor-guide.md](batch-extractor-guide.md) | 下游之二：场景 id 池的两阶段维护（`ResizableGPUBuffer` + `GrowableRangeAllocator` 的直接用户） |

阅读源码时建议按 `buffer/mod.rs` → `abstract_resource.rs` → `linear_buffer_array/` → `allocator/` 的顺序，与本文的展开顺序一致。

## 模式概览

整个缓冲资源层回答三个递进的问题：

- **"一块 GPU 缓冲"的通用形态是什么**：`AbstractBuffer` trait 把"一块可读写的 GPU 存储"抽象成字节级的统一接口（读、写、resize、搬迁、拷贝、绑定），实际实现可以是真 GPU buffer，也可以是纹理模拟的"假缓冲"。任何下游代码只需要面对 `Box<dyn AbstractBuffer>`，不关心背后是哪种平台资源。
- **"缓冲怎么创建"**：`AbstractStorageAllocator` trait 把"创建一块存储缓冲"抽象成可插拔的分配策略——默认策略创建真 GPU buffer（`DefaultStorageAllocator`），GLES-only 平台可以换成 texture-as-buffer 策略，需要合并小缓冲时可以换成合并分配器，下游代码完全无感。
- **"缓冲怎么被维护"**：`linear_buffer_array` 的组合子体系定义"线性存储"（一维 `[T]` 数组）的一系列能力 trait（容量查询、下标读写、resize、增长策略、宿主备份），并用包装类型逐层叠加行为；`allocator/` 子层在这之上管理"谁占用哪个槽位"。

一句话概括分层关系：

```text
GPUBuffer（wgpu 原生缓冲 + usage）
  └─ 类型化容器：StorageBufferReadonlyDataView<[T]> / StorageBufferDataView<[T]> / UniformBufferDataView<T>（storage.rs / uniform.rs）
       └─ AbstractBuffer（字节级统一接口，DynTypedStorageBuffer 是动态类型实现）
            └─ AbstractStorageAllocator（创建策略：默认 / texture-as-buffer / 合并缓冲）
                 └─ linear_buffer_array 组合子（ResizableGPUBuffer / GPUStorageDirectQueueUpdate /
                    VecWithStorageBuffer / BufferWidthDefaultValue / CustomGrowBehaviorMaintainer）
                      └─ allocator/（GPUSlatAllocateMaintainer / GPURangeAllocateMaintainer 槽位管理）
                           └─ 下游：SparseUpdateStorageBuffer（hook-utils）、SceneModelListPool（batch-extractor）、
                               SBT 池（ray-tracing）、task pool（task-graph）……
```

## 核心概念

| 概念 | 定义位置 | 说明 |
| --- | --- | --- |
| `GPUBufferResourceView` | [buffer/mod.rs:19](../../platform/graphics/webgpu/src/resource/buffer/mod.rs#L19) | wgpu 缓冲 + 资源描述 + 视图（offset/size 区间），整个缓冲层的最终载体 |
| `AbstractBuffer` | [abstract_resource.rs:175](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L175) | 缓冲抽象 trait：字节单位，resize / write / relocate / copy / 双端绑定 |
| `BufferRelocate` | [abstract_resource.rs:168](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L168) | 一次搬迁描述（self_offset → target_offset + count，字节单位） |
| `AbstractStorageAllocator` | [abstract_resource.rs:3](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L3) | 存储分配抽象：给定字节数与类型描述创建缓冲 |
| `AbstractStorageAllocatorExt` | [abstract_resource.rs:57](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L57) | 类型化便捷入口：`allocate` / `allocate_readonly` / `allocate_readonly_init` |
| `DefaultStorageAllocator` | [abstract_resource.rs:135](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L135) | 默认分配策略：真 GPU storage buffer |
| `DynTypedStorageBuffer` | [abstract_resource.rs:305](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L305) | 动态类型缓冲：GPU 视图 + 类型描述 + 只读标记 |
| `AbstractStorageBuffer<T>` / `AbstractReadonlyStorageBuffer<T>` | [abstract_resource.rs:399](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L399) / [:435](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L435) | 类型化包装：Deref 到 `BoxedAbstractBuffer`，绑定出 `ShaderPtrOf<T>` / `ShaderReadonlyPtrOf<T>` |
| `LinearStorageBase` | [linear_buffer_array/mod.rs:35](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/mod.rs#L35) | 线性存储基 trait：`Item: Pod` + `max_size()`（item 计数） |
| `GPULinearStorage` | [linear_buffer_array/mod.rs:14](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/mod.rs#L14) | GPU 线性存储：暴露 `gpu()` 与 `abstract_gpu()`，组合子 `with_direct_resize` / `with_queue_direct_update` 的入口 |
| `ResizableLinearStorage` / `RelocationResizableLinearStorage` | [linear_buffer_array/mod.rs:107](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/mod.rs#L107) / [:91](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/mod.rs#L91) | resize 能力：`resize` / `grow_at_least` / `resize_with_relocations` |
| `LinearStorageDirectAccess` / `LinearStorageViewAccess` | [linear_buffer_array/mod.rs:62](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/mod.rs#L62) / [:163](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/mod.rs#L163) | 下标写入（含字节级部分写）与只读视图访问 |
| `ResizableGPUBuffer<T>` | [gpu_raw.rs:3](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/gpu_raw.rs#L3) | `with_direct_resize` 包装：按 item 计数 resize，独立 encoder 提交 |
| `GPUStorageDirectQueueUpdate<T>` | [queue_direct_update.rs:3](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/queue_direct_update.rs#L3) | `with_queue_direct_update` 包装：写入走 `queue.write_buffer` 直写 |
| `VecWithStorageBuffer<T>` | [vec_backup.rs:3](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/vec_backup.rs#L3) | `with_vec_backup` 包装：宿主 Vec 备份（diff 可选），`view()` 返回宿主数据 |
| `BufferWidthDefaultValue<T>` | [default_value.rs:3](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/default_value.rs#L3) | `with_default_value_with_init_write` 包装：resize 增长区间自动填默认值 |
| `CustomGrowBehaviorMaintainer<T>` | [grow_behavior.rs:11](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/grow_behavior.rs#L11) | `with_default_grow_behavior` / `with_grow_behavior` 包装：插入增长策略，越界写自动增长 |
| `GrowableDirectQueueUpdateBuffer<T>` / `GrowableHostedDirectQueueUpdateBuffer<T>` | [linear_buffer_array/mod.rs:170](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/mod.rs#L170) / [:184](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/mod.rs#L184) | 两个常用组合产物（无/有宿主备份） |
| `AllocatorStorageBase` / `LinearAllocatorStorage` / `RangeAllocatorStorage` | [allocator/mod.rs:14](../../platform/graphics/webgpu/src/resource/buffer/allocator/mod.rs#L14) / [:33](../../platform/graphics/webgpu/src/resource/buffer/allocator/mod.rs#L33) / [:42](../../platform/graphics/webgpu/src/resource/buffer/allocator/mod.rs#L42) | 槽位分配器 trait：单槽（slab）与连续区间（range）两种语义 |
| `GPUSlatAllocateMaintainer<T>` | [allocator/slab.rs:3](../../platform/graphics/webgpu/src/resource/buffer/allocator/slab.rs#L3) | slab 分配器实现（`slab::Slab<()>` 管槽位，缓冲管数据） |
| `GPURangeAllocateMaintainer<T>` | [allocator/range.rs:5](../../platform/graphics/webgpu/src/resource/buffer/allocator/range.rs#L5) | 连续区间分配器实现（`GrowableRangeAllocator` + offset→id 反查表） |
| `GrowableRangeAllocator<K>` | [utility/growable-range-allocator/src/lib.rs:7](../../utility/growable-range-allocator/src/lib.rs#L7) | 通用可增长范围分配器本体（xalloc TLSF 封装），`GPURangeAllocateMaintainer` 与 hook-utils 批分配共用 |

## 地基：GPUBuffer 与类型化容器

[mod.rs:19](../../platform/graphics/webgpu/src/resource/buffer/mod.rs#L19) 的 `GPUBufferResourceView = ResourceViewRc<GPUBuffer>` 是整个缓冲层的最终载体：`GPUBuffer` 持有 `Arc<gpu::Buffer>`，资源描述里带 `usage` 与 `size`，视图描述 `GPUBufferViewRange` 带 `offset` / `size`（字节）。它既是可绑定资源（`BindableResourceProvider`），也是所有类型化容器解包的底层。

在它之上是三个类型化容器（[storage.rs](../../platform/graphics/webgpu/src/resource/buffer/storage.rs)、[uniform.rs](../../platform/graphics/webgpu/src/resource/buffer/uniform.rs)，详见 shader-edsl-binding-and-typed-container-zh）：

- `StorageBufferReadonlyDataView<T>` / `StorageBufferDataView<T>`：`T: Std430MaybeUnsized`，usage 恒为 `STORAGE | COPY_DST | COPY_SRC`。`[T]` 形态提供 `item_count()`；原子场景可 `into_device_atomic_array()` 换 `[DeviceAtomic<T>]` 视图。
- `UniformBufferDataView<T>`：`T: Std140`，usage 为 `UNIFORM | COPY_DST`，`write_at` 直写；`UniformBufferCachedDataView` 带 CPU 侧 diff 缓存。

这两类容器（连同它们内部持有的 `GPUBufferResourceView`）在 [abstract_resource.rs:496](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L496) 与 [:569](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L569) **直接实现 `AbstractBuffer`**——所以它们是"具体容器"与"抽象缓冲"的桥：既可以被类型化绑定，也可以作为字节级抽象被 resize / relocate。

## AbstractBuffer：字节级的缓冲抽象

[abstract_resource.rs:175](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L175) 定义整层的核心 trait，**所有单位都是字节**：

```rust
pub trait AbstractBuffer: DynClone + Send + Sync {
  fn byte_size(&self) -> u64;
  fn resize_gpu(&mut self, encoder: &mut GPUCommandEncoder, device: &GPUDevice,
                new_byte_size: u64, relocations: Option<&mut dyn Iterator<Item = BufferRelocate>>) -> bool;
  fn write(&self, content: &[u8], offset: u64, queue: &GPUQueue);
  fn batch_self_relocate(&self, iter: &mut dyn Iterator<Item = BufferRelocate>,
                         encoder: &mut GPUCommandEncoder, device: &GPUDevice);
  fn copy_buffer_to_buffer(&self, target: &dyn AbstractBuffer, self_offset: u64,
                           target_offset: u64, count: u64, encoder: &mut GPUCommandEncoder);
  fn bind_shader(&self, bind_builder: &mut ShaderBindGroupBuilder) -> BoxedShaderPtr;
  fn bind_pass(&self, bind_builder: &mut BindingBuilder);
  fn as_any(&self) -> &dyn Any;
  fn get_gpu_buffer_view(&self) -> Option<GPUBufferResourceView>;
}
```

各方法职责：

- `resize_gpu`：把缓冲扩容/缩容到精确的 `new_byte_size`（字节），返回是否成功。`relocations` 可选：传入时在 resize 过程中**一并应用搬迁**（见下文的 `resize_impl`）。**全量拷贝旧内容到新缓冲**是 resize 的一部分——这是"resize 不丢数据"的保证，即使提供了 relocations 也不能跳过（代码注释明确说明：无法保证 relocations 覆盖整个缓冲，见 [abstract_resource.rs:657](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L657)）。
- `write`：queue 直写一段字节到 offset。这是唯一的"小写"通道（配合组合子层做逐槽位写入）。
- `batch_self_relocate`：**同一个缓冲内部**的批量搬迁。因为源与目标在同一块缓冲上、区间可能重叠，实现（`batch_relocate_impl`，[abstract_resource.rs:272](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L272)）先把整块拷进一个临时 COPY_DST|COPY_SRC 缓冲，再按 relocations 从临时缓冲拷回自身——源数据永远来自临时副本，重叠搬迁也安全。`DynTypedStorageBuffer` 的注释解释了为什么要单独设计这个接口：抽象缓冲不能深拷贝自己（clone 是引用语义），而批量搬迁中"自己"可能被重叠引用。
- `copy_buffer_to_buffer`：**不同缓冲之间**的拷贝（目标必须是不同类型的缓冲实例，不能是自身的 ref clone）。
- `bind_shader` / `bind_pass`：动态类型的双端绑定。`DynTypedStorageBuffer` 的实现（[abstract_resource.rs:339](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L339)）用 `binding_dyn` 走动态类型绑定，`writeable_if_storage: !readonly`——所以"只读标记"决定着色器侧拿到的是读写指针还是只读指针。
- `get_gpu_buffer_view`：返回底层的 `GPUBufferResourceView`。**返回 None 说明这个"缓冲"其实是纹理模拟的**（见 texture-as-buffer 实现）——这是稀疏写选择"compute 复制 or queue 直写"的分叉点（见 webgpu-hook-utils-guide 的 write_abstract）。
- `as_any`：向下转型，供需要"同一类型"的操作（如 texture-as-buffer 的 copy_buffer_to_buffer 要求目标也是纹理缓冲）使用。

### resize 与 relocate 的落地细节

`resize_impl`（[abstract_resource.rs:643](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L643)）是 resize 的通用实现，所有基于 `GPUBufferResourceView` 的实现共用：

```text
new_byte_size > max_buffer_size → 返回 false（不创建）
创建同 usage 的 zeroed 新缓冲（create_gpu_buffer_zeroed）
encoder.copy_buffer_to_buffer：旧缓冲 [0, min(old, new)) → 新缓冲 [0, ...)   // 全量拷贝，数据保持
若有 relocations：逐条 old_offset → new_offset 拷贝                         // 搬迁与 resize 合一
*self = new_buffer                                                          // 换视图
```

关键点：**resize 与 relocations 合一**，只做一次"全量拷贝 + 增量搬迁"，这正是范围分配器扩容时"避免额外一次独立搬迁拷贝"的设计（`RelocationResizableLinearStorage::resize_with_relocations` 的文档注释，[linear_buffer_array/mod.rs:91](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/mod.rs#L91)）。

`GPUBufferResourceView` 的视图区间（`desc.offset`）在 resize 中也被正确换算：`resize_impl` 拷贝整块底层资源，relocations 的源加 `buffer.desc.offset`、目标加新视图的 offset（DynTypedStorageBuffer 的 batch_self_relocate 同理）。

## AbstractStorageAllocator：可插拔的分配策略

[abstract_resource.rs:3](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L3) 把"创建缓冲"抽象成策略：

```rust
pub trait AbstractStorageAllocator: DynClone + Send + Sync {
  fn allocate_dyn_ty(&self, byte_size: u64, device: &GPUDevice,
                     ty_desc: MaybeUnsizedValueType, readonly: bool, label: &str) -> BoxedAbstractBuffer;
  fn get_layout(&self) -> StructLayoutTarget;   // 布局目标（Std430 / Packed），供宿主按布局计算 stride
  fn is_readonly(&self) -> bool;                // 该策略是否只能分配只读缓冲
}
```

- `ty_desc: MaybeUnsizedValueType` 是着色器类型描述（见 shader-edsl-core-zh），`allocate_dyn_ty` 必须把"类型 + 字节数"打包进返回的缓冲，保证之后 `bind_shader` 能按正确类型绑定。
- `get_layout` 很重要：缓冲只是字节，类型布局（Std430 / Packed）决定宿主侧怎么解释字节（例如 task-graph 用它算 task 结构体的 stride，见下文）。
- 分配缓冲是"一次性"的：后续 resize 走 `AbstractBuffer::resize_gpu`，不需要再经过分配器。

`AbstractStorageAllocatorExt`（[abstract_resource.rs:57](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L57)）是类型化便捷层：

- `allocate<T>`：`T::maybe_unsized_ty()` 取类型描述，`readonly = false`，返回 `AbstractStorageBuffer<T>`（读写）。
- `allocate_readonly<T>`：`readonly = true`，返回 `AbstractReadonlyStorageBuffer<T>`（**下游 99% 的用法**——只读存储缓冲是 GPU 侧数据表的标准形态）。
- `allocate_readonly_init<T>`：额外用 `value.bytes()` 初始化内容（`buffer.write(value, 0, &gpu.queue)`）。

两个类型化包装 `AbstractStorageBuffer<T>` / `AbstractReadonlyStorageBuffer<T>`（[:399](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L399) / [:435](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L435)）就是 `PhantomData<T>` + `BoxedAbstractBuffer`：Deref 到抽象层，`bind_shader` 产出 `ShaderPtrOf<T>` / `ShaderReadonlyPtrOf<T>`（类型化着色器指针），`[T]` 形态额外提供 `item_count()`。所有下游拿到手的基本就是这个形态的句柄。

### 默认实现与替代策略

- `DefaultStorageAllocator`（[abstract_resource.rs:135](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L135)）：创建 `StorageBufferReadonlyDataView<[u32]>`（STORAGE | COPY_DST | COPY_SRC 的 zeroed 缓冲）包成 `DynTypedStorageBuffer`。代码注释坦诚"类型标记与读写标记在这里其实用不上"——因为真 GPU buffer 的绑定能力来自 wgpu，动态类型只对纹理模拟路径才有意义。
- `TextureAsStorageAllocator`（[webgpu-texture-as-buffer/src/lib.rs:15](../../platform/graphics/webgpu-texture-as-buffer/src/lib.rs#L15)）：GLES-only 平台的"犯罪级 hack"——把只读存储缓冲用 **R32Uint 纹理**实现（一行数据一行纹素，宽度 = max_texture_dimension_2d）。实现要点：
  - 数据先落在宿主备份 `data: Vec<u8>` 里，`bind_pass` 时 `check_update_texture` 才把脏行区域用 `queue.write_texture` 刷进纹理（[lib.rs:160](../../platform/graphics/webgpu-texture-as-buffer/src/lib.rs#L160)）。
  - 着色器侧 `bind_shader` 用 `AbstractPtr` 机制把"纹理 + 宽度"伪装成一个 `[u32]` 堆指针（`TextureAsU32Heap`，下标 → (x, y) 纹素坐标），数组长度存在纹理第 0 个纹素里（所以宿主备份在 offset 0 前留了 4 字节存 array_len）。
  - `resize_gpu` 只改宿主备份并标记脏区间；`batch_self_relocate` 直接操作宿主 Vec（写 4 处偏移 + 4 是因为前面留了 array_len 字节）。
  - `get_gpu_buffer_view` 返回 `None`——这正是稀疏写退化为 queue 直写的判据（webgpu-hook-utils-guide 的 write_abstract 分叉）。
  - 它的存在意义：在 GLES-only 平台上原型验证间接渲染（配合 MIDC 降级与存储缓冲自动合并）。
- `CombinedStorageBufferAllocator`（[webgpu-virtual-typed-combine-buffer/src/storage.rs:4](../../platform/graphics/webgpu-virtual-typed-combine-buffer/src/storage.rs#L4)）：把很多小存储缓冲**合并进一块大缓冲**（子缓冲按类型布局 Packed/Std430 排布），返回 `SubCombinedStorageBufferDynTyped` 作为 `AbstractBuffer`。它内部再包一个底层 allocator（`new` 的 `internal_allocator` 参数），子缓冲 resize 后整个合并缓冲需要重建（`check_rebuild`）。当前仓库内没有外部消费者，属于实验性实现，但它是"allocator 可插拔"这一设计最极端的例证。

### 策略怎么被选中

分配器在 viewer 的 `RenderingContent::storage_allocator()`（[application/viewer-content/src/rendering/frame_all.rs:551](../../application/viewer-content/src/rendering/frame_all.rs#L551)）按配置二选一：`using_texture_as_storage_buffer_for_indirect_rendering` 开启（GLES-only 平台）时返回 `TextureAsStorageAllocator`，否则 `DefaultStorageAllocator`。之后 `QueryGPUHookCx`（webgpu-hook-utils）把 `Box<dyn AbstractStorageAllocator>` 挂在上下文里，**整个渲染侧的所有 `allocate_readonly` 都经由它创建**——换平台后端不需要改任何下游代码。这就是该抽象的核心价值。

## linear_buffer_array 组合子体系

[linear_buffer_array/mod.rs](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/mod.rs) 定义一组"线性存储"能力 trait，以及用包装类型叠加行为的组合子。核心思想：**底层是一个 `[T]` 数组（item 为单位），各种维护行为（resize、增长策略、宿主备份、直写通道、默认值填充）都是可以按任意顺序叠加的装饰器**。

### 能力 trait 族

| trait | 能力 | 关键方法 |
| --- | --- | --- |
| `LinearStorageBase` | 基础：item 类型与容量 | `Item: Pod`、`max_size()`（**item 计数**） |
| `LinearStorageDirectAccess` | 下标写入 | `set_value` / `set_values` / `remove` / `removes` / `set_value_sub_bytes`（字节级部分写，安全前提是入参在界内） |
| `LinearStorageViewAccess` | 只读视图 | `view() -> &[Item]`（宿主备份时返回备份数据） |
| `ResizableLinearStorage` | resize | `resize`（精确容量）/ `grow_at_least`（至少容量，默认委托 resize）/ `with_grow_behavior` / `with_default_grow_behavior` |
| `RelocationResizableLinearStorage` | resize + 搬迁合一 | `resize_with_relocations(new_size, relocations)` |
| `GPULinearStorage` | GPU 接入 | `gpu() -> &GPUType`、`abstract_gpu() -> &mut dyn AbstractBuffer`，以及组合子入口 `with_direct_resize` / `with_queue_direct_update` |

`max_size` 全部以 **item 计数**计量，字节换算只发生在最底层的 `ResizableGPUBuffer` / `AbstractBuffer` 边界——上层（分配器、调用方）全程以 item 思考，避免字节/元素换算错误。

### 五个组合子

组合子都是"实现能力 trait 的包装结构体"，每个解决一类维护行为：

| 组合子 | 包装类型 | 行为 |
| --- | --- | --- |
| `with_direct_resize(gpu)` | `ResizableGPUBuffer<T>`（[gpu_raw.rs:3](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/gpu_raw.rs#L3)） | 把 `LinearStorageBase` 的 item 计数换算成字节调 `AbstractBuffer::resize_gpu`；用**自己的独立 encoder** 创建并提交（`ctx.create_encoder()`），不占用帧 encoder |
| `with_queue_direct_update(queue)` | `GPUStorageDirectQueueUpdate<T>`（[queue_direct_update.rs:3](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/queue_direct_update.rs#L3)） | 所有 `set_value*` 变成 `queue.write_buffer` 逐槽直写；`remove` 是空操作（"归零由上层控制"） |
| `with_vec_backup(none_default, diff)` | `VecWithStorageBuffer<T>`（[vec_backup.rs:3](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/vec_backup.rs#L3)） | 宿主侧维护 `vec: Vec<Item>`（初始全部填 `none_default`）；`diff=true` 时写入前与备份比较、相同则跳过（避免重复直写）；`resize` 同步 `vec.resize(new_size, none_default)`；`view()` 返回宿主 Vec——**这就是 host-driven 路径现场读回数据的通道** |
| `with_default_value_with_init_write(default)` | `BufferWidthDefaultValue<T>`（[default_value.rs:3](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/default_value.rs#L3)） | 构造时把整个初始容量写成 `default`；之后每次 **增长** resize 只把新增区间填 `default`（缩容不填） |
| `with_default_grow_behavior(max)` / `with_grow_behavior(f)` | `CustomGrowBehaviorMaintainer<T>`（[grow_behavior.rs:11](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/grow_behavior.rs#L11)） | 插入增长策略：`set_value` 等写入发现 `idx + 1 > max_size` 时自动调策略函数扩容后再写（"unbound mutation 自动增长"）；`grow_at_least(required)` 也是入口。默认策略 `(current * 2).max(required).min(max)`，`required > max` 时返回 `None` 拒绝扩容 |

### CustomGrowBehaviorMaintainer 的细节

[grow_behavior.rs:11](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/grow_behavior.rs#L11) 是最常用的组合子（`SparseUpdateStorageBuffer` 的外层就是它）。语义要点：

- 策略函数签名 `Fn(ResizeInput { current_size, required_size }) -> Option<u32>`：输入当前容量与需求容量，输出目标容量（`None` = 拒绝扩容，调用方收到失败）。
- `check_resize`（[grow_behavior.rs:42](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/grow_behavior.rs#L42)）：**只有 `max_size() < required` 才调用策略**；已满足需求时是 no-op，绝不缩容。
- 写入路径（`set_value` / `set_values` / `set_value_sub_bytes`）都会自动 `check_resize(idx + 1)`——所以对"槽位下标 = 数据库分配索引"的场景（见下），**写入天然触发扩容**，无需调用方显式 resize。
- `grow_at_least` 走 `check_resize`；`resize` 直通内层（精确容量，不经过策略）——外部分配器（如范围分配器）缩容时用这个。

### 组合示例与常用产物

组合是任意的，但仓库里有两个标准配方（[linear_buffer_array/mod.rs:170](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/mod.rs#L170) 起）：

```rust
// 无宿主备份：queue 直写 + 自动增长
pub type GrowableDirectQueueUpdateBuffer<T> =
  CustomGrowBehaviorMaintainer<GPUStorageDirectQueueUpdate<ResizableGPUBuffer<T>>>;

// 带宿主备份：在直写基础上再包 Vec 备份（diff 可选）
pub type GrowableHostedDirectQueueUpdateBuffer<T> =
  CustomGrowBehaviorMaintainer<VecWithStorageBuffer<GPUStorageDirectQueueUpdate<ResizableGPUBuffer<T>>>>;
```

以及对应的工厂函数 `create_growable_buffer` / `create_growable_buffer_with_host_back`（[mod.rs:173](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/mod.rs#L173) / [:188](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/mod.rs#L188)）。

组合子的顺序有讲究：**`with_vec_backup` 必须在 `with_queue_direct_update` 之外**（Vec 备份先于直写更新自身，且 `view()` 返回宿主数据），**`with_default_grow_behavior` 永远在最外层**（增长策略要看到完整的 max_size 链）。`view-dependent-transform` 的四层叠加是完整示范（[extension/view-dependent-transform/src/indirect_draw.rs:84](../../extension/view-dependent-transform/src/indirect_draw.rs#L84)）：

```rust
let buffer = alloc.allocate_readonly(make_init_size::<u32>(128), &gpu.device, "...");
let index_remap = buffer
  .with_direct_resize(gpu)              // 1. item → 字节，独立 encoder resize
  .with_queue_direct_update(&gpu.queue) // 2. 写入走 queue 直写
  .with_default_value_with_init_write(u32::MAX) // 3. 初始化与扩容区间填 u32::MAX（= 无效句柄）
  .with_default_grow_behavior(u32::MAX);        // 4. 无上限自动增长
```

### 缓冲生命周期与数据保持语义

把以上所有机制合起来，一个"常驻 GPU 的线性缓冲"的生命周期是：

```text
创建：AbstractStorageAllocator::allocate_readonly（初始容量，可选 allocate_readonly_init 带初值）
  └─ 写入：逐槽 set_value*（queue 直写 / compute 稀疏写，依组合而定）
  └─ 增长：
       ├─ 自动：CustomGrowBehaviorMaintainer 在越界写时按策略扩容（set_value 路径）
       ├─ 显式：grow_at_least(required)（容量同步，如 use_max_item_count_by_db_entity）
       └─ 分配器驱动：范围分配器 update 后按 resize_to 调 resize_with_relocations
            └─ 数据保持：resize 全量拷贝旧内容；relocations 在 resize 中一并应用（避免二次拷贝）
  └─ 重排：batch_self_relocate（同缓冲内搬迁，临时缓冲防重叠）
  └─ 销毁：随持有者 drop（GPU 资源经 ResourceRc 引用计数释放）
```

两条值得强调的语义：

- **resize 永远不丢数据**（全量拷贝），所以"扩容后内容还在"是稳定契约；但要写入新区域必须先知道它已被默认值覆盖——用 `with_default_value_with_init_write` 或 `with_vec_backup(none_default)` 的 `none_default` 保证"新槽位 = 哨兵值"。
- **resize 用独立 encoder 立即提交**（`ResizableGPUBuffer`），不依赖帧 encoder——这允许 spawn 阶段（worker 线程）直接扩容；而帧 encoder 上的写入（稀疏写）发生在 render 阶段，两者顺序由两阶段模型的 token 保证（见 webgpu-hook-utils-guide）。

## allocator 子层：槽位分配器

[allocator/mod.rs](../../platform/graphics/webgpu/src/resource/buffer/allocator/mod.rs) 定义"分配器"能力 trait 与 `RelocationMessage { previous_offset, new_offset }`（搬迁通知，宿主用来更新自己的偏移映射）：

```rust
pub trait AllocatorStorageBase: LinearStorageBase {
  fn current_used(&self) -> u32;
  fn try_reserve_used(&mut self, used: u32, relocation_handler: ...) { /* 默认空 */ }
  fn try_compact(&mut self, relocation_handler: ...) { /* 默认空 */ }
}
pub trait LinearAllocatorStorage: AllocatorStorageBase {
  fn deallocate(&mut self, idx: u32);
  fn allocate_value(&mut self, v: Self::Item) -> Option<u32>;
  fn deallocate_back(&mut self, idx: u32) -> Option<Self::Item>;  // 取出旧值并释放
}
pub trait RangeAllocatorStorage: AllocatorStorageBase {
  fn deallocate(&mut self, idx: u32);
  fn allocate_values(&mut self, v: &[Self::Item], relocation_handler) -> Option<u32>;
  fn allocate_range(&mut self, count: u32, relocation_handler) -> Option<u32>;
}
```

两个 trait 对应两种槽位语义：**单槽分配**（每个实体一个 item，删除可复用，像 slab）与**连续区间分配**（每个实体一段连续 item，可增长，会整体搬迁）。`try_reserve_used` / `try_compact` 是预留的"预扩容 / 紧凑化"接口，默认空实现（注意：预扩容成功不保证区间分配一定成功——碎片问题，注释里写明了这只是性能考虑）。

### GPUSlatAllocateMaintainer：slab 分配器

[allocator/slab.rs:3](../../platform/graphics/webgpu/src/resource/buffer/allocator/slab.rs#L3) 用 `slab::Slab<()>` 管"哪个槽位被占用"（`insert` 复用最小空闲下标），缓冲管数据：

- `allocate_value(v)`：`allocator.insert(())` 拿下标 → `buffer.set_value(idx, v)`，失败则返回 None（注释约定"底层负责 resize 并传播 resize 失败"——即配合 `CustomGrowBehaviorMaintainer` 自动增长）。注意 slab 槽位在 set_value 失败时不会回滚（当前无人触发）。
- `deallocate(idx)`：`allocator.remove` + `buffer.remove(idx)`（后者由 Vec 备份层写回 `none_default`）。
- `deallocate_back(idx)`：取出旧值并释放——**注意这个方法的实现有逻辑问题**（见 doc/task.md 的「文档编写中发现的重要逻辑问题」）。

便捷产物（[slab.rs:92](../../platform/graphics/webgpu/src/resource/buffer/allocator/slab.rs#L92) 起）：

```rust
pub type StorageBufferSlabAllocatePool<T> = SlabAllocatePool<StorageBufferReadonlyDataView<[T]>>;
pub type SlabAllocatePool<T> = GPUSlatAllocateMaintainer<GrowableDirectQueueUpdateBuffer<T>>;
```

### GPURangeAllocateMaintainer：连续区间分配器

[allocator/range.rs:5](../../platform/graphics/webgpu/src/resource/buffer/allocator/range.rs#L5) 封装 `GrowableRangeAllocator<u32>`（[utility/growable-range-allocator/src/lib.rs:7](../../utility/growable-range-allocator/src/lib.rs#L7)，xalloc TLSF 分配器 + 增长/收缩策略）。与 hook-utils 的批分配用法不同，这里的用法是**同步的**：分配器内部 key 是单调递增的 id，调用方拿到的句柄是**区间偏移**（offset），所以维护一张 `offset_to_id` 反查表：

- `allocate_range(count, relocation_handler)`（[range.rs:61](../../platform/graphics/webgpu/src/resource/buffer/allocator/range.rs#L61)）：`next_id += 1` 取新 id → `allocator.update([], [(id, count)])`（新 key 不需要先释放）→ 失败记入 `failed_to_allocate` 返回 None → `apply_resize_and_relocations` → 返回 `new_data_to_write` 里的 offset，并登记 `offset_to_id`。
- `apply_resize_and_relocations`（[range.rs:30](../../platform/graphics/webgpu/src/resource/buffer/allocator/range.rs#L30)）：有 `resize_to` 时把 `data_movements`（item 单位）换算成字节的 `BufferRelocate`，调 `buffer.resize_with_relocations`（数据保持 + 搬迁合一，见上文）；同时更新 `offset_to_id`（old → new）并逐条回调 `relocation_handler`——**调用方借此更新自己宿主侧的偏移映射**（SBT 的 offset map 就是这么维护的）。
- `deallocate(offset)`：反查 id → `get_region` 取 (size, offset) → `allocator.update([id], [])` 释放 → `buffer.removes(offset, size)` 把整段写回默认值。
- `allocate_values(v, handler)`：`allocate_range(v.len())` 后 `buffer.set_values`。

便捷产物（[range.rs:153](../../platform/graphics/webgpu/src/resource/buffer/allocator/range.rs#L153) 起）：

```rust
pub type StorageBufferRangeAllocatePool<T> = RangeAllocatePool<AbstractReadonlyStorageBuffer<[T]>>;
pub type RangeAllocatePool<T> = GPURangeAllocateMaintainer<GrowableDirectQueueUpdateBuffer<T>>;
pub fn create_storage_buffer_range_allocate_pool(gpu, allocator, label, init_item_count, max_item_count);
```

### 与 GrowableRangeAllocator 本体的分工

同一份 `GrowableRangeAllocator` 核心有两层用法，容易混淆：

| 用法 | 位置 | 语义 |
| --- | --- | --- |
| 批分配（两阶段） | webgpu-hook-utils 的 `use_range_allocated_device_buffers`、batch-extractor 的 `SceneModelListPool`、multi_access | spawn 阶段 `update` 一次拿到 `BatchAllocateResult`（removed / failed / data_movements / new_data_to_write / resize_to 四类互斥变更），`BatchAllocateResultShared::apply_resize` 在 render 前应用 resize，变更包跨阶段传输 |
| 同步分配 | webgpu 层 `GPURangeAllocateMaintainer`（SBT 等非 hook 场景） | 每次 allocate / deallocate 立刻调用 `update`、立刻 resize / 写缓冲，搬迁逐条回调 `relocation_handler` |

两者的 `update` 契约一致：**`new` 里出现的 key 必须同时在 `change_or_removed_keys` 里（先释放再分配）**，debug 断言强制（[growable-range-allocator/src/lib.rs:96](../../utility/growable-range-allocator/src/lib.rs#L96)）；`maybe_shrink`（[:191](../../utility/growable-range-allocator/src/lib.rs#L191)）在利用率低于一半时缩到 `used * 2`（对齐要求 > 1 时跳过，因为对齐后的实际占用可能超过名义 used 导致搬迁失败）。

## 与下游的衔接

### webgpu-hook-utils：两阶段稀疏写

- `SparseUpdateStorageBuffer<T>` 的缓冲就是三层组合（[webgpu-hook-utils/src/sparse_update_storage_buffer.rs:8](../../platform/graphics/webgpu-hook-utils/src/sparse_update_storage_buffer.rs#L8)）：`CustomGrowBehaviorMaintainer<ResizableGPUBuffer<AbstractReadonlyStorageBuffer<[T]>>>`——`allocate_readonly` 创建、`with_direct_resize` 可扩容、`with_default_grow_behavior(max)` 自动增长。它**没有** `with_queue_direct_update`：写入走的是 `use_update` 的 compute 稀疏写（帧 encoder），`use_update_impl` 的 render 分支对 `buffer.abstract_gpu()` 调 `write_abstract`（[:139](../../platform/graphics/webgpu-hook-utils/src/sparse_update_storage_buffer.rs#L139)）。`use_max_item_count_by_db_entity` 调 `grow_at_least`——槽位下标 = 数据库实体分配索引，容量与表容量同步。
- `SparseUpdateStorageWithHostBuffer<T>` 多一层 `with_vec_backup(T::zeroed(), false)` 并整体包 `Arc<RwLock>`（[:72](../../platform/graphics/webgpu-hook-utils/src/sparse_update_storage_buffer.rs#L72)）：`use_update` 时把同一份稀疏更新写进宿主 Vec（`write_sparse_updates`），渲染侧经 `make_read_holder`（utility/query 的 `LockReadGuardHolder`）现场读回数据生成 DrawCommand（宽线参数、attribute mesh 元数据等）。
- `use_range_allocated_device_buffers`（[webgpu-hook-utils/src/lib.rs:48](../../platform/graphics/webgpu-hook-utils/src/lib.rs#L48)）是"范围分配器 + 组合子 + 两阶段"的完整模板：`allocate_readonly` + `with_direct_resize` 建池，spawn 阶段 worker 线程里 `allocator.update` → `RangeAllocateBufferCollector::prepare` 打包 → `BatchAllocateResultShared::apply_resize(&mut *gpu_buffer.write())` **此刻就 resize**（独立 encoder，spawn 阶段允许），render 阶段 `write` 落地数据与分配结果。

### batch-extractor：GPU id 池

`SceneModelListPool<K>`（[batch-extractor/src/list_pool.rs:29](../../scene/rendering/batch-extractor/src/list_pool.rs#L29)）：`allocate_readonly(init * 4).with_direct_resize(gpu)`——**没有增长组合子**，扩容由 `update_pool_size` 显式 `resize`（容量由范围分配器的 `resize_to` 决定），且对齐要求取 `min_storage_buffer_offset_alignment.max(min_uniform_buffer_offset_alignment) / 4`（u32 单位，保证每个子列表区域能切成合法 buffer view）。这正是"用 `with_direct_resize` 而不用 `with_default_grow_behavior`"的范例：容量策略由外部分配器（`GrowableRangeAllocator`）决定，组合子只提供"精确 resize"能力。id 池的两阶段维护细节见 batch-extractor-guide。

### ray-tracing：SBT 的同步分配

[shader/ray-tracing/src/backend/wavefront_compute/sbt.rs:78](../../shader/ray-tracing/src/backend/wavefront_compute/sbt.rs#L78) 是 webgpu 层分配器（非 hook 路径）的最大消费者：

- `meta: StorageBufferSlabAllocatePoolWithHost<DeviceSBTTableMeta>`——slab 池 + `GrowableHostedDirectQueueUpdateBuffer`（`create_growable_buffer_with_host_back(gpu, buffer, max_size, true)`，[:281](../../shader/ray-tracing/src/backend/wavefront_compute/sbt.rs#L281)）：每类光线的 SBT 记录一个元数据槽，槽位复用、删除写回零值。
- `ray_hit / ray_miss / ray_gen: StorageBufferRangeAllocatePool<...>`——三个区间池存各类型着色器记录。
- `allocate` 时对三个区间池分别 `allocate_range(ray_type_count, relocation_handler)`，回调里用 `set_value_sub_bytes` 把新偏移写进 meta 槽的对应字段（字节级部分写！），并更新宿主 offset map；`deallocate` 时 `meta.deallocate_back(id)` 取出旧 meta，据此释放三个区间（[:199](../../shader/ray-tracing/src/backend/wavefront_compute/sbt.rs#L199)）。

这是"webgpu 缓冲层 + 同步分配器"完整用法的教科书：slab 管元数据、range 管变长记录、`RelocationMessage` 回调解宿主映射、`set_value_sub_bytes` 做字段级更新。

### 其他直接消费者

- **task-graph** 的 `TaskPoolAllocation`（[shader/task-graph/src/runtime/task_pool.rs:48](../../shader/task-graph/src/runtime/task_pool.rs#L48)）：`allocator.get_layout()` 算任务结构体 stride → `allocate_dyn_ty` 分配可写任务池缓冲（**readonly = false 的罕见用法**）。
- **webgpu-midc-downgrade**（[platform/graphics/webgpu-midc-downgrade/src/host_driven.rs:54](../../platform/graphics/webgpu-midc-downgrade/src/host_driven.rs#L54)）：`allocate_readonly_init` 一次性创建带初始数据的 draw command 缓冲。
- **gpu-indirect** 的 attribute mesh（[scene/rendering/gpu-indirect/src/shape/attribute/mod.rs:205](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L205)）：顶点/索引池 `allocate_readonly` + `with_direct_resize`，与 hook-utils 批分配配合（见 attribute-mesh-indirect-render-guide）。

## 使用规则

1. **缓冲创建一律走 `AbstractStorageAllocator`**（通常来自 `QueryGPUHookCx` 或 GPU 初始化参数），不要直接 `GPUBuffer::create`——否则会绕过平台后端切换（texture-as-buffer）能力。只读存储缓冲用 `allocate_readonly`，带初值用 `allocate_readonly_init`。
2. **槽位下标 = 实体分配索引时，容量必须与数据库表容量同步**（`grow_at_least` / `use_max_item_count_by_db_entity`），写入前保证容量是调用方契约；`CustomGrowBehaviorMaintainer` 只在有增长组合子时自动兜底。
3. **resize 是"全量拷贝 + 可选搬迁"**：数据保持是稳定的；需要"新槽位 = 哨兵"语义时用 `with_default_value_with_init_write` 或 `with_vec_backup(none_default)`，不要假设 GPU 新缓冲是零值（其实是零值，但语义上依赖组合子更清晰——尤其 texture 模拟路径）。
4. **同缓冲内搬迁用 `batch_self_relocate`，跨缓冲拷贝用 `copy_buffer_to_buffer`**（目标必须是不同类型实例）；两者都在传入的 encoder 上记录命令。
5. **组合子顺序**：`with_direct_resize` 最先、`with_vec_backup` 在 `with_queue_direct_update` 之外、`with_default_grow_behavior` 最外；`resize` 与 `grow_at_least` 语义不同（精确 vs 至少），外部分配器驱动的扩容用 `resize_with_relocations`。
6. **分配器契约**：`GrowableRangeAllocator::update` 要求 `new` 中的 key 必须已出现在 `change_or_removed_keys`（先释放再分配）；`failed_to_allocate` 可能包含此前分配成功的 key；分配结果四类变更互斥。
7. **`relocation_handler` / `RelocationMessage` 必须同步宿主偏移映射**——SBT 的 offset map、hook-utils 的 `offset_to_id` 都靠它保持与 GPU 侧一致；漏更新会导致释放/读取错位。

## 常见疑问

- **为什么 `AbstractBuffer` 全部用字节而 `LinearStorageBase` 全部用 item**：抽象层要容纳任意实现（纹理模拟、合并缓冲）与任意类型，字节是唯一通用单位；组合子层面向"按 item 寻址的数组"场景，换算集中在 `ResizableGPUBuffer` 一处，避免上层反复换算出错。
- **`with_queue_direct_update` 与稀疏写（compute）有什么区别**：前者每个 `set_value` 一次 `queue.write_buffer`（宿主侧直写，适合小规模/低频），后者把一帧所有碎片写合并成一次 compute 派发（适合每帧大量槽位更新）；`SparseUpdateStorageBuffer` 刻意不挂直写组合子，写入只走帧 encoder。
- **`with_vec_backup` 的 `view()` 为什么可靠**：Vec 备份与 GPU 是同一套布局、同一写入入口（`VecWithStorageBuffer` 的 `set_value*` 先写 Vec 再传内层），`resize` 同步补默认值——所以渲染时读备份等价于读 GPU 内容，只是少一次回读。
- **什么时候用 slab、什么时候用 range**：槽位等宽、需要复用（实体删除后新实体顶替）用 slab；槽位变长、需要连续区域（SBT 记录、池化顶点）用 range。两者都支持"失败返回 None"（分配失败由上层决定跳过或重试）。
- **`DynTypedStorageBuffer` 的 `ty` 标记有什么用**：对真 GPU buffer，绑定能力来自 wgpu 的 binding 描述，类型标记"其实用不上"（DefaultStorageAllocator 注释的原话）；但对纹理模拟路径，`bind_shader` 必须知道类型才能生成"伪指针"（texture-as-buffer 的 `TextureU32HeapPtrWithType`）——这正是动态类型存在的原因。

## 延伸阅读

- 下游文档：本层是 [webgpu-hook-utils-guide.md](webgpu-hook-utils-guide.md)（稀疏写 / 批分配封装）与 [batch-extractor-guide.md](batch-extractor-guide.md)（id 池两阶段维护）的直接地基
- 范围分配器本体与批量变更：[utility/growable-range-allocator/src/lib.rs](../../utility/growable-range-allocator/src/lib.rs)
- 纹理模拟缓冲：[platform/graphics/webgpu-texture-as-buffer/src/lib.rs](../../platform/graphics/webgpu-texture-as-buffer/src/lib.rs)
- 合并缓冲（实验性）：[platform/graphics/webgpu-virtual-typed-combine-buffer/src/storage.rs](../../platform/graphics/webgpu-virtual-typed-combine-buffer/src/storage.rs)
- 类型系统与绑定：[skill-translation/shader-edsl-binding-and-typed-container-zh.md](skill-translation/shader-edsl-binding-and-typed-container-zh.md)、[skill-translation/shader-edsl-core-zh.md](skill-translation/shader-edsl-core-zh.md)
