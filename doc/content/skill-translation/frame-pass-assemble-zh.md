---
name: frame-pass-assemble
description: >
  关于在 rendiation 中组装多通道渲染帧的参考。涵盖 pass()、attachment()、render_ctx()、
  by()、by_if()、FrameCtx、PassContent、UseQuadDraw 以及颜色/深度载入-存储(load-store)
  操作。在帧中编排渲染通道时使用 —— 组合几何体、后处理、多重采样抗锯齿(MSAA)解析与
  帧拷贝,构成完整的 GPU 帧。
  关于底层抽象与实现(RenderComponent、ShaderHashProvider、ShaderPassBuilder、便捷包装器),
  参见 fundamental-gpu-component-model。
metadata:
  version: "1.0"
  updated: "2026-05-16"
---

rendiation 中的多通道渲染帧组装。该 API 是一个函数式/可链式构建器,用于组合 GPU 渲染通道。关键文件:

| 文件 | 用途 |
|------|------|
| [platform/graphics/webgpu/src/frame/pass.rs](../../../../../rendiation/platform/graphics/webgpu/src/frame/pass.rs) | `pass()`、`RenderPassDescription`、`ActiveRenderPass`、`PassContent`、`FrameRenderPass` |
| [platform/graphics/webgpu/src/frame/attachment.rs](../../../../../rendiation/platform/graphics/webgpu/src/frame/attachment.rs) | `attachment()`、`AttachmentDescriptor`、`PooledTextureKey` |
| [platform/graphics/webgpu/src/frame/mod.rs](../../../../../rendiation/platform/graphics/webgpu/src/frame/mod.rs) | `FrameCtx` |
| [platform/graphics/webgpu/src/frame/quad.rs](../../../../../rendiation/platform/graphics/webgpu/src/frame/quad.rs) | `UseQuadDraw`、`QuadDraw<T>` |
| [platform/graphics/webgpu/src/pass.rs](../../../../../rendiation/platform/graphics/webgpu/src/pass.rs) | `RenderTargetView`、`GPURenderPassCtx` |

关于 `RenderComponent`、`ShaderHashProvider`、`ShaderPassBuilder` 与便捷包装器,参见 `fundamental-gpu-component-model`。

## FrameCtx —— 帧宿主

`FrameCtx` 持有单个帧的 GPU 命令编码器、纹理池与帧状态。所有通道组装都发生在接收 `&mut FrameCtx` 的代码中。

```rust
// 由框架提供 —— 你接收 ctx,而不是创建它。
// 关键字段:
ctx.gpu          // &GPU
ctx.frame_size   // Size
// ctx.scope(f) 创建带有全新 hook 内存的子作用域
```

## pass() —— 创建渲染通道

`pass(name)` 返回一个 `RenderPassDescription` 构建器。

```rust
pass("my-pass")                     // RenderPassDescription
    .with_color(&target, op)        // 推入一个颜色附件(支持多个,按顺序)
    .with_depth(&depth, d_op, s_op) // 设置深度模板(可选)
    .render_ctx(ctx)                // 启动 GPU 通道 → ActiveRenderPass
```

### 颜色附件操作

| 函数 | 行为 |
|----------|----------|
| `store_full_frame()` | 载入时清除,结束时存储;必须确保存储覆盖整个帧 |
| `load_and_store()` | 保留已有内容,结束时存储 |
| `load_once_and_discard()` | 保留内容,通道结束后丢弃 |
| `clear_and_store(v)` | 清除为指定值,结束时存储 |

### 深度附件操作

相同的函数,分别应用于深度与模板操作:
```rust
.with_depth(&depth_view, load_and_store(), load_and_store())
//                        ^ 深度操作         ^ 模板操作
```

### 解析多重采样抗锯齿(MSAA)

```rust
pass("resolve")
    .with_color_and_resolve_target(
        &msaa_target,              // 4x 多重采样抗锯齿(MSAA)
        load_once_and_discard(),   // 消费 MSAA,然后丢弃
        &single_sample_target,     // 解析目标(1x)
    )
    .render_ctx(ctx);
```

### 带副作用的无操作通道

没有任何 `.by()` 调用的通道仍然会执行——对多重采样抗锯齿解析或清理附件很有用。

## attachment() —— 分配瞬态纹理

`attachment()` 返回一个 `AttachmentDescriptor` 构建器。纹理从帧持久化的池中分配,并在帧间自动复用。

```rust
attachment()
    .format(TextureFormat::Rgba16Float)  // 默认:Rgba8UnormSrgb
    .sample_count(4)                      // MSAA
    .sizer(ratio_sizer(0.5))              // 半分辨率
    .request(ctx)                         // → RenderTargetView
```

### 构建器方法

| 方法 | 默认值 | 描述 |
|--------|---------|-------------|
| `.format(f)` | `Rgba8UnormSrgb` | 纹理格式 |
| `.sample_count(n)` | `1` | 多重采样抗锯齿(MSAA)采样数 |
| `.sizer(f)` | 恒等 | 尺寸变换,例如 `ratio_sizer(0.5)` |
| `.extra_usage(flags)` | —— | 额外的 `TextureUsages` |
| `.use_hdr_if_enabled(bool)` | —— | 开启 HDR 时切换为 `Rgba16Float` |

### 预构建快捷方式

```rust
attachment()           // 默认颜色
depth_attachment()     // 默认深度
```

### 跨通道复用同一附件

```rust
let target = attachment().request(ctx);
// ... 在同一帧内的多个通道中使用 &target
```

## render_ctx() 与 ActiveRenderPass

`.render_ctx(ctx)` 消费 `RenderPassDescription`,启动真正的 GPU 渲染通道,并返回 `ActiveRenderPass`。

```rust
let active = pass("name")
    .with_color(&target, store_full_frame())
    .render_ctx(ctx);
// active: ActiveRenderPass
```

在 `render_ctx()` 之前对描述对象调用 `.make_all_channel_and_depth_into_load_op()`,可以把所有操作改为 `Load`(当你想保留上一个通道的输出而不重新清除时很有用)。

## by() —— 把内容渲染进通道

`.by(content)` 把一个 `PassContent` 实现渲染进当前通道。返回 `Self`,因此可以链式调用。

```rust
pass("geometry")
    .with_color(&target, store_full_frame())
    .with_depth(&depth, load_and_store(), load_and_store())
    .render_ctx(ctx)
    .by(&mut draw_skybox)       // 渲染天空盒
    .by(&mut draw_geometry)     // 渲染主体几何体
    .by(&mut draw_transparent); // 渲染透明物体
// drop 时提交通道
```

## by_if() —— 条件渲染

`.by_if(&mut option)` 仅在 `option` 为 `Some` 时渲染。

```rust
let mut compose = pass("compose")
    .with_color(&final_target, load_and_store())
    .render_ctx(ctx)
    .by_if(&mut self.tonemap)      // 可选的色调映射
    .by_if(&mut self.highlight);   // 可选的高光叠加
```

## PassContent trait

任何通过 `.by()` 渲染的类型。两条主要路径:

```rust
// QuadDraw<T> —— 全屏四边形(后处理中最常见)
// 任何 RenderComponent 都能通过 blanket impl 免费获得 .draw_quad():
my_post_processor.draw_quad()
my_shader.draw_quad_with_alpha_blending()
my_shader.draw_quad_with_blend(Some(my_blend_state))

// 几何绘制 —— 包在 RenderVec/RenderSlice 中的 RenderComponent
// 通过 DrawCommand 使用顶点/索引缓冲区绘制
```

### PassContent 与 RenderComponent —— 设计分工

这两个 trait 处理不同粒度的抽象:

- **`PassContent`** —— 通道内**绘制工作**的单位。它代表一块逻辑上独立的业务渲染逻辑。它接收 `FrameRenderPass`,并且可能发出**多次绘制调用**,每次由通过 `RenderArray`/`RenderSlice` 组装的一个或多个 `RenderComponent` 支撑。它决定在通道内*画什么、按什么顺序画*。

- **`RenderComponent`** —— 单次绘制调用的**着色器逻辑**单位。它提供 GPU 管线中可复用、可独立缓存的部分:着色器定义、管线哈希与资源绑定。它没有通道或绘制命令的概念——这些由调用方提供。`RenderSlice`/`RenderArray` 也可以作为 `RenderComponent` 的组合子,通过洋葱模型把多个组件嵌套成一个。

一个 `PassContent` 实现通常会组装一个 `DefaultPassDispatcher`(每通道状态)加上一个或多个 `RenderComponent`,对每次绘制调用 `RenderComponent::render()`,并控制绘制顺序。`QuadDraw<T>` 是 `PassContent` 的一个例子——它把单个 `RenderComponent` 包进全屏四边形——但任何多绘制模式(几何通道:天空盒 + 主体 + 透明物体;OIT:深度预通道 + 颜色 + 解析)也都是 `PassContent`。

关于组件模型的细节(`ShaderHashProvider`、`ShaderPassBuilder`、`GraphicsShaderProvider`、便捷包装器、洋葱中间件模型),参见 `fundamental-gpu-component-model`。

## 完整帧示例

源自 [application/viewer-content/src/rendering/frame_viewport.rs](../../../../../rendiation/application/viewer-content/src/rendering/frame_viewport.rs)(以下为简化示意;该文件的当前实现已重构为包含 TAA、SSAO 与 GBuffer 的流程;下方用到的每个 API 仍然有效):

```rust
fn render(&mut self, ctx: &mut FrameCtx) {
    // 分配附件
    let scene_result = attachment()
        .sample_count(4)
        .request(ctx);
    let depth = depth_attachment()
        .sample_count(4)
        .request(ctx);
    let simple_sample = attachment().request(ctx); // 用于解析的 1x 目标

    // 通道 1:用多重采样抗锯齿渲染几何体
    pass("scene")
        .with_color(&scene_result, store_full_frame())
        .with_depth(&depth, load_and_store(), load_and_store())
        .render_ctx(ctx)
        .by(&mut self.skybox)
        .by(&mut self.geometry);

    // 通道 2:解析 MSAA
    pass("resolve")
        .with_color_and_resolve_target(
            &scene_result,
            load_once_and_discard(),
            &simple_sample,
        )
        .render_ctx(ctx);

    // 通道 3:后处理
    let final_target = attachment().request(ctx);
    pass("post")
        .with_color(&final_target, store_full_frame())
        .render_ctx(ctx)
        .by(&mut PostProcess {
            input: simple_sample,
        }.draw_quad());
}
```

## 关键类型速查

| 类型 | 角色 |
|------|------|
| `FrameCtx` | 每帧的 GPU 状态、编码器、池、内存 |
| `RenderPassDescription` | 通道构建器:名称 + 颜色/深度附件 |
| `ActiveRenderPass` | 运行中的通道:链式 `.by()` 调用 |
| `AttachmentDescriptor` | 纹理分配构建器 |
| `RenderTargetView` | 已分配的纹理句柄(来自池) |
| `PassContent` | trait:任何可以渲染进通道的东西 |
| `GPURenderPassCtx` | 每通道渲染状态(绑定、管线) |
| `RenderComponent` | 着色器 + 哈希 + 设置 = 可渲染实体 |
| `UseQuadDraw` | blanket impl:任何 T → `QuadDraw<T>`,用于全屏四边形 |
