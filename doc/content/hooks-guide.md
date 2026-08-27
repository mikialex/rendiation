# Hooks 模式理解（utility/hook）

本文档是对 [utility/hook](../../utility/hook) 实现的模式梳理，聚焦 hook 运行时本身：状态寻址、作用域、执行轮次与状态生命周期。不涉及增量查询与多阶段调度，那是 [query-hook-guide.md](query-hook-guide.md) 的内容。

## 模式概览

这个 crate 把 React hooks 的思想搬到 Rust 渲染引擎里：

- 逻辑组织成"每轮执行、带持久状态"的函数，类似 React 组件的 render。
- 状态没有名字，身份是"调用点 + 调用序号 + 类型"，按调用顺序隐式寻址。
- 子逻辑与分支通过 scope 获得独立记忆——**scope 的核心目的就是处理分支**，父子的状态序号互不干扰。
- 执行分"动态阶段"与"静态阶段"：前者允许建立新的状态形状，后者只能按既定形状访问，形状不一致立刻 panic。

## 核心概念

### HooksCxLike trait

[utility/hook/src/hooks.rs](../../utility/hook/src/hooks.rs) 的 `HooksCxLike` 是每个 Cx 都必须实现的接口，只要求四件事：

| 方法 | 含义 |
| --- | --- |
| `memory_mut` / `memory_ref` | 访问本 Cx 的 FunctionMemory |
| `flush` | 每轮执行结束时的收尾（由实现决定何时真正清理） |
| `is_dynamic_stage` | 当前是否处于允许创建新状态的阶段 |

trait 自带一组基于 FunctionMemory 的通用方法：`execute`、`scope`、`keyed_scope`、`skip_if_not`、`use_plain_state` 系列等，实现者无需重复。

`execute` 是标准执行入口：运行闭包、重置 cursor 与 scope_index、标记 created、调用 flush。

### FunctionMemory：按序寻址的状态存储

FunctionMemory 是核心数据结构：

- 状态用 bumpalo 的 Bump 分配（批量回收，无逐对象释放）。
- `expect_state_init` 按 `current_cursor`（当前调用序号）寻址：首次调用到该位置时执行 init 并登记（值指针、类型、cleanup/drop 函数），之后直接复用。
- debug 构建下会做类型匹配校验，错位时 panic 并打印期望/实际类型名。
- 子作用域存于 `sub_functions` / `sub_functions_next` 两张表中（见下文 flush）。

因此状态是"调用点 + 调用序号 + 类型"三者的函数，同一 hook 函数每次执行时 **use_xxx 的调用顺序必须稳定**。

### scope：子函数拥有独立的记忆（处理分支的核心机制）

`cx.scope(...)` 以"调用点 Location + scope_index"为 key 换取一块独立的 FunctionMemory，子函数内部的 `use_plain_state` 顺序与父函数互不影响。实现上 `raw_scope` 直接 swap 当前 memory 指针，把子记忆顶到工作位，执行完再换回。

**scope 的核心目的是处理分支**：hook 函数内出现条件分支时，每个分支的状态归属由分支所在 scope 的 key（调用点 + scope_index）决定，而不是外层调用序列的顺序槽位。这样分支切换（条件变化导致走不同分支）不会使状态错位——每个分支的记忆各自独立、按 key 稳定寻址。

- `keyed_scope`：以用户提供的 key（Hash 值）替代调用点寻址，用于循环、动态列表等调用点不固定的场景，每个迭代的身份由 key 稳定；实现上把 key 的 hash 字节收集进 `UserDefined` 变体。
- `next_scope_index`：手动推进 scope 序号，用于同一调用点内多个不同分支/子逻辑的区分。
- `skip_call_site_scope` / `skip_keyed_scope`：跳过时仍然登记一个空子作用域，占住位置（配合 skip_if_not 保持形状）。

### 动态阶段与静态阶段

`is_dynamic_stage()` 决定当前执行是否允许创建新状态：

- 动态阶段：`sub_function` 从 `sub_functions` 中取出已有记忆，或创建新的放入 `sub_functions_next`。重复创建同一 key 会 panic。
- 静态阶段：作用域必须已存在于 `sub_functions` 中，找不到直接 panic。

这套机制保证 hook 图（哪个调用点有状态）在动态阶段定型，后续静态执行只能按既定形状访问。若静态阶段出现"上一轮没有的调用点"，说明逻辑分支在变化，问题当场暴露而不是静默错位。

`skip_if_not(should_execute, f)` 与之配合：它内部就是 `scope(f)`，条件不满足时跳过整段逻辑（用 skip_call_site_scope 占位），而动态创建阶段即使条件为假也必须执行以登记状态形状（`must_execute = 动态阶段 && 尚未创建`）。当整段逻辑可以安全跳过时才用它省去执行成本；分支本身的状态隔离由 scope 承担（skip_if_not 内部已用 scope 包住被跳过的逻辑）。

### flush 与 cleanup：状态生命周期

- `flush(drop_cx)`：每轮执行结束时调用。把本轮未访问的旧子作用域 drain 出来做清理，再与 `sub_functions_next` 交换，使记忆与最新执行路径对齐。`flush_assume_only_plain_states` 是只含普通状态（无 cleanup 依赖）时的轻量版。
- `cleanup(drop_cx)`：销毁全部状态（依次调用 cleanup_fn 再 drop），用于 Cx 退出时（见 [application/viewer/src/app_loop.rs](../../application/viewer/src/app_loop.rs) 中窗口关闭时 `self.memory.cleanup(...)`）。`cleanup_assume_only_plain_states` 为轻量版。

### DynCx：类型擦除的动态上下文

[utility/hook/src/dyn_cx.rs](../../utility/hook/src/dyn_cx.rs) 提供一个按类型寻址的指针栈：

- `register_cx` / `unregister_cx` / `scoped_cx`：把 `&mut T` 压入/弹出，`get_cx_ref` / `get_cx_mut` 取当前栈顶。同一类型可多层注册（栈式）。
- `access_cx!` / `access_cx_mut!` 宏提供 guard 化的安全访问（内部是 unsafe 指针操作）。
- `MessageStore`：按 TypeId 存单份消息，`put` / `get` / `take`。

典型用途：把需要跨模块共享的运行时对象（如 ViewerDataScheduler）在窗口生命周期内注册，子逻辑按类型取用。

## 使用规则

以下规则是使用该框架编写 hook 逻辑时必须遵守的约定，违反大多导致 panic 或状态错位。

1. **状态 hook 的调用顺序必须在函数每次执行中保持一致**。带分支的逻辑用 scope 隔离分支（见规则 2），不要在同一调用序列里直接内联会随分支变化的状态 hook。
2. **分支/子逻辑用 `cx.scope(...)` 包裹，循环与动态 key 场景用 `keyed_scope`**。这是处理分支的核心机制：分支内的状态顺序独立于外部，分支切换不破坏形状；同一调用点多次进入不同子逻辑时用 `next_scope_index` 区分。`skip_if_not` 不是分支处理的替代品，它是"整段逻辑可安全跳过"时的优化（内部也是 scope）。
3. **循环或动态 key 场景必须用 `keyed_scope`**。例如 per-item 状态以 item 身份作 key，否则各迭代共享同一调用点会互相覆盖。
4. **新状态只能在动态阶段首次创建**。静态阶段遇到未创建的状态或作用域会 panic：不要在静态阶段引入新的 use_xxx 调用。若某段逻辑可能被跳过（如未唤醒的分支），用 `skip_if_not` 包住（内部 scope 保证形状不变），并保证它在动态阶段被执行过。
5. **普通状态用 `use_plain_state`**（init 闭包或 Default）；需要跨函数共享的可变状态用 `use_sharable_plain_state`（Arc<RwLock<T>>）。
6. **需要清理的状态实现 `CanCleanUpFrom<DropCx>`，用 `use_state_init` 注册**；无清理需求的值用 `NothingToDrop` 包装（参照 [ViewerCx::use_state_init](../../application/viewer/src/viewer/mod.rs) 的写法）。cleanup 在 flush（作用域结束）与 memory cleanup（Cx 退出）时执行。
7. **scope 的 key 由调用点 + scope_index 组成**，两个不同调用点的 scope 天然独立；不要依赖调试信息中的地址做跨轮持久身份（FastLocation 按指针相等比较）。
8. **DynCx 的注册与注销必须配对**（`scoped_cx` 自动处理）；同一类型栈式注册时，取到的总是最近压入的实例。

## Cx 实现要点

实现 `HooksCxLike` 需要给出四件事（参照 [application/viewer-content-api/src/cx.rs](../../application/viewer-content-api/src/cx.rs)）：

- memory 的读写访问（Cx 结构里持有 `&mut FunctionMemory`）。
- `flush`：决定何时真正清理——实现通常只在动态阶段执行 `memory.flush(drop_cx)`，静态阶段为空操作。
- `is_dynamic_stage`：哪些阶段允许创建状态。
- `use_plain_state`：通常经 `expect_state_init` 包装 `NothingToDrop(f())` 实现。

`HooksCxLike` 是 `unsafe` trait：实现者需保证 memory 指针在 Cx 生命周期内有效，且 `use_plain_state` 返回的 `&mut T` 不与 `&mut Self` 别名（标准做法是先取原始指针再重新借用，参照各 Cx 实现与 tests 中的写法）。
