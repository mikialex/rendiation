---
name: fundamental-gpu-component-model
description: >
  关于 rendiation 中可组合 GPU 组件模型的参考。涵盖 RenderComponent、ShaderHashProvider、
  ShaderPassBuilder、GraphicsShaderProvider 以及便捷包装器(RenderVec、RenderSlice、
  RenderArray、OptionRender、BindingController)。在定义参与管线缓存、资源绑定与绘制派发
  系统的可渲染实体时使用。
metadata:
  version: "1.0"
  updated: "2026-05-16"
---

可组合的 GPU 组件模型是 rendiation 中可渲染实体(renderable entity)的核心抽象。一个类型通过实现三个 trait——`ShaderHashProvider`、`GraphicsShaderProvider` 与 `ShaderPassBuilder`——即可变得可渲染。框架会对同时满足这三个 trait 的任何类型自动推导出 `RenderComponent`,使其获得完整的渲染路径:管线缓存(pipeline caching)、资源绑定(resource binding)与绘制派发(draw dispatch)。

关键文件:

| 文件 | 用途 |
| ------ | ------ |
| [platform/graphics/webgpu/src/rendering.rs](../../../../../rendiation/platform/graphics/webgpu/src/rendering.rs) | `RenderComponent` 与便捷包装器 |
| [platform/graphics/webgpu/src/pass.rs](../../../../../rendiation/platform/graphics/webgpu/src/pass.rs) | `ShaderPassBuilder`、`GPURenderPassCtx`、`GPURenderPass` |
| [platform/graphics/webgpu/src/device.rs](../../../../../rendiation/platform/graphics/webgpu/src/device.rs) | `ShaderHashProvider`、`PipelineHasher`、管线缓存 |
| [platform/graphics/webgpu/src/frame/pass_base.rs](../../../../../rendiation/platform/graphics/webgpu/src/frame/pass_base.rs) | `DefaultPassDispatcher` |
| [shader/api/src/graphics/mod.rs](../../../../../rendiation/shader/api/src/graphics/mod.rs) | `GraphicsShaderProvider` |

## 三个超级 trait

```rust
// 对任何 T: ShaderHashProvider + GraphicsShaderProvider + ShaderPassBuilder 自动实现
pub trait RenderComponent: ShaderHashProvider + GraphicsShaderProvider + ShaderPassBuilder { ... }
```

一个类型**不**显式实现 `RenderComponent`。它实现三个组成 trait,由 blanket impl 自动提供 `RenderComponent`。

## ShaderHashProvider —— 管线缓存键

**文件**: [platform/graphics/webgpu/src/device.rs](../../../../../rendiation/platform/graphics/webgpu/src/device.rs)

```rust
pub trait ShaderHashProvider {
    fn hash_pipeline(&self, _hasher: &mut PipelineHasher) {}
    fn hash_type_info(&self, hasher: &mut PipelineHasher);
    fn hash_pipeline_with_type_info(&self, hasher: &mut PipelineHasher) {
        self.hash_type_info(hasher);
        self.hash_pipeline(hasher);
    }
}
```

两个哈希钩子:

- `hash_type_info` —— **必需**。对类型的结构身份(通常是 `TypeId`)求哈希。这保证不同类型的组件即使哈希出相同的数据,也绝不会在缓存中互相冲突。
- `hash_pipeline` —— 有默认实现可选覆盖,但**只要有任何数据会影响 `build()` / `post_build()` 中生成的着色器代码,就必须重写它**。

最终的 `u64` 哈希用作 `GPUDevice.render_pipeline_cache: HashMap<u64, GPURenderPipeline>` 的键。缓存未命中时,通过 `GraphicsShaderProvider::build_self` 触发着色器编译。

### `hash_pipeline` 的正确性规则

**`hash_pipeline` 必须对每一份可能影响编译结果 `GPURenderPipeline` 的数据求哈希。**

一个 `GPURenderPipeline` 把编译好的着色器程序与所有固定功能管线状态(混合模式、深度模板、颜色目标格式、采样数、图元拓扑等)烘焙在一起。两类信息都必须被哈希覆盖:

- **着色器代码** —— 由于 `build()` / `post_build()` 是用 EDSL 发出着色器逻辑的过程式 Rust 代码,任何会改变生成着色器输出的 Rust 控制流(`if`、`match`、循环、配置标志)都必须被哈希。
- **管线状态** —— 在 `build()` 或 `post_build()` 内部设置到 `ShaderRenderPipelineBuilder` 上的任何东西:颜色目标格式、深度模板配置、采样数、混合状态、图元状态等。

如果两个组件实例产生了不同的着色器逻辑或管线状态,却返回了相同的哈希,缓存会返回错误的管线——导致绘制损坏、绑定布局不匹配或 WGPU 校验错误。

一份最小检查清单:

- 在 `build()` 或 `post_build()` 内部产生分支的状态枚举 / 标志
- 通过 `builder.define_out_by()` 注册的颜色/深度格式与采样数
- 混合状态、深度模板操作、图元状态(剔除模式等)
- 任何影响绑定描述符生成的运行时配置

拿不准就哈希它。一次虚假的缓存未命中只多花一次编译;一次虚假的缓存命中则会破坏渲染。

### 辅助宏:`shader_hash_type_id!`

```rust
// 最常见的模式 —— 用 TypeId::of::<Self>() 作为 hash_type_info
impl ShaderHashProvider for MyComponent {
    shader_hash_type_id!();
    fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
        self.some_config.hash(hasher);
    }
}
```

### 全覆盖实现

- `impl ShaderHashProvider for ()` —— 空操作(`hash_type_info` 与 `hash_pipeline` 都不做任何事)
- `impl<T: ShaderHashProvider> ShaderHashProvider for &T` —— 委托给 `T`

## ShaderPassBuilder —— 资源绑定设置

**文件**: [platform/graphics/webgpu/src/pass.rs](../../../../../rendiation/platform/graphics/webgpu/src/pass.rs)

```rust
pub trait ShaderPassBuilder {
    fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {}
    fn post_setup_pass(&self, ctx: &mut GPURenderPassCtx) {}

    fn setup_pass_self(&self, ctx: &mut GPURenderPassCtx) {
        self.setup_pass(ctx);
        self.post_setup_pass(ctx);
    }
}
```

- `setup_pass` —— 在管线**绑定之前**调用。在这里通过 `ctx.binding` 绑定纹理、缓冲区、采样器。
- `post_setup_pass` —— 在管线**绑定之后**调用。很少需要;用于那些必须等到知道管线布局之后才能确定的绑定。

`ctx` 是 `&mut GPURenderPassCtx`,它提供:

```rust
pub struct GPURenderPassCtx {
    pub pass: GPURenderPass,        // wgpu 渲染通道(Deref 到 gpu::RenderPass<'static>)
    pub gpu: GPU,                   // 设备 + 队列访问
    pub binding: BindingBuilder,    // 为该通道累积绑定组
    incremental_vertex_binding_index: u32,
    pub enable_bind_check: bool,
}
```

### 绑定契约:setup 必须匹配 build,post_setup 必须匹配 post_build

`ShaderPassBuilder` 中的绑定声明与 `GraphicsShaderProvider` 中的着色器绑定声明构成一份严格契约,**必须始终一致**:

| 设置侧 | 构建侧 |
| ----------- | ------------ |
| `setup_pass()` | `build()` |
| `post_setup_pass()` | `post_build()` |

这些规则对**两个**配对同样适用:

- **顺序** —— 资源必须按与着色器侧声明完全一致的顺序绑定。绑定组索引 N 对应着色器绑定 N。
- **类型** —— 每个绑定的类型(纹理、统一缓冲区、存储缓冲区、采样器)必须与对应的着色器声明匹配。
- **动态分支** —— 如果 `build()` 或 `post_build()` 使用 Rust 控制流有条件地声明绑定,对应的 `setup_pass()` 或 `post_setup_pass()` 必须遵循**相同的控制流与相同的条件**。

任何不匹配都会在绘制时导致 WGPU 校验错误。当绑定动态变化时,动态因素必须在 `hash_pipeline` 中求哈希——否则会复用带有不同绑定布局的缓存管线,造成绑定索引或类型不匹配。

### 典型实现

```rust
impl ShaderPassBuilder for MyEffect {
    fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
        // 顺序与类型必须与 build() 声明的内容完全一致:
        ctx.binding.bind(&self.texture);   // 匹配 build() 中的绑定 0
        ctx.binding.bind(&self.sampler);   // 匹配 build() 中的绑定 1
        ctx.binding.bind(&self.uniform);   // 匹配 build() 中的绑定 2
    }
}
```

全覆盖实现:`impl ShaderPassBuilder for ()`、`impl<T: ShaderPassBuilder> ShaderPassBuilder for &T`。

## GraphicsShaderProvider —— 着色器定义

**文件**: [shader/api/src/graphics/mod.rs](../../../../../rendiation/shader/api/src/graphics/mod.rs)

```rust
pub trait GraphicsShaderProvider {
    fn build(&self, _builder: &mut ShaderRenderPipelineBuilder) {}
    fn post_build(&self, _builder: &mut ShaderRenderPipelineBuilder) {}

    fn build_self(
        &self,
        api_builder: &dyn Fn(ShaderStage) -> DynamicShaderAPI,
        info: Arc<GPUInfo>,
        checks: ShaderRuntimeChecks,
    ) -> Result<ShaderRenderPipelineBuilder, Vec<ShaderBuildError>> { ... }

}
```

- `build` —— 注册顶点/片元着色器阶段。最先被调用。
- `post_build` —— 补充最终默认值(例如自动向片元输出 0 写入白色)。在 `build` 之后调用。
- `build_self` —— 编排完整的构建:创建构建器、调用 `build`、调用 `post_build`、收集错误。

关于如何在 `build` 内部编写着色器代码,详见 `shader-edsl-graphics` skill。

## RenderComponent::render 的工作方式

当 `RenderComponent::render(ctx, draw)` 被调用时(`draw: RenderMethod` 决定绘制方式),框架按以下顺序执行:

- **哈希** —— 调用 `self.hash_pipeline_with_type_info(hasher)` 得到缓存键;当 `draw` 为 `RenderMethod::MeshPipelineDraw` 时,还会把网格渲染逻辑的哈希一并混入
- **查找或构建管线** —— `ctx.gpu.device.get_or_cache_create_render_pipeline(hasher, |device| { ... })`
  - 缓存未命中:调用 `self.build_self(...)` → 依次触发 `build()` 与 `post_build()`(洋葱先正向再反向)
  - 缓存命中:返回缓存的管线
- **重置状态** —— 重置 `ctx.binding` 并复位顶点绑定索引
- **绑定管线** —— 将编译好的 `GPURenderPipeline` 绑定到 wgpu 渲染通道上;若 `draw` 为 `MeshPipelineDraw`,还会调用 `draw.bind_shader(ctx)` 绑定网格着色器
- **绑定检查** —— 若已启用,校验累积的绑定组布局与管线期望的布局是否一致
- **Setup pass** —— 调用 `self.setup_pass_self(ctx)`,依次执行 `setup_pass()` 与 `post_setup_pass()`。对于 `RenderSlice`,洋葱遍历在这里发生:`setup_pass` 正向(A→B→C),随后 `post_setup_pass` 反向(C→B→A)
- **刷新绑定** —— `ctx.binding.setup_render_pass(&mut ctx.pass, &ctx.gpu.device, &pipeline)` 把所有累积的绑定组提交到 wgpu 渲染通道
- **绘制** —— `RenderMethod::TraditionalDraw` 时调用 `ctx.pass.draw_by_command(draw_cmd)` 发出 GPU 绘制调用;`RenderMethod::MeshPipelineDraw` 时调用 `ctx.pass.dispatch_mesh_draw_command(draw.dispatch_command())` 派发网格绘制命令

以上全部发生在 `ActiveRenderPass` 上的一次 `.by()` 调用之内。

`RenderMethod` 是 `render` 的第二个参数,决定绘制方式:

| 变体 | 用途 |
| --------- | ----- |
| `RenderMethod::TraditionalDraw(DrawCommand)` | 传统的顶点/索引缓冲区绘制(详见下文 DrawCommand 变体) |
| `RenderMethod::MeshPipelineDraw(&dyn MeshComponent)` | 网格管线绘制:使用 `MeshComponent`(网格着色器逻辑 + 派发命令)完成 GPU 驱动的网格绘制 |

## 洋葱模型 —— 通过 `RenderSlice` 做中间件组合

`GraphicsShaderProvider` 与 `ShaderPassBuilder` 中的 `pre`(正向)/`post`(反向)方法对,是为了让组件以**洋葱(中间件)模式**组合而设计的。当组件 `[A, B, C]` 被包进一个 `RenderSlice` 时,执行顺序是:

```
    ┌──────────────────────────┐
    │  A                       │
    │    ┌──────────────────┐  │
    │    │ B                │  │
    │    │   ┌──────────┐   │  │
    │    │   │ C (core) │   │  │
    │    │   └──────────┘   │  │
    │    └──────────────────┘  │
    └──────────────────────────┘
```

**着色器构建**(先 `build` 再 `post_build`):

```
build:      A → B → C    (外层先注册)
post_build: C → B → A    (内层先收尾)
```

**通道设置**(先 `setup_pass` 再 `post_setup_pass`):

```
setup_pass:      A → B → C    (外层先绑定)
post_setup_pass: C → B → A    (内层先清理 / 覆盖)
```

外层组件可以包装或覆盖内层组件的行为:

- 在 `build` 中,A 注册基础通道配置,B 在其上追加,C 提供最终的着色器逻辑。
- 在 `post_build` 中,C 添加自己的输出默认值,B 包装它们,A 应用最终的全局覆盖(例如自动写入白色)。
- 在 `setup_pass` 中,A 绑定全局资源(视口 uniform),B 绑定自己的纹理,C 绑定自己的特定数据。
- 在 `post_setup_pass` 中,C 可以发出绘制后的清理,然后是 B,最后是 A。

正向传递从最外层到最内层构建上下文;反向传递从最内层到最外层展开——就像 HTTP 框架中的中间件栈或 Rust 的 `tower::Service` 层。

便捷包装器(`RenderSlice`、`RenderArray`、`RenderVec`)就是实现这一模式的组合子——它们对 `pre` 方法按正序调用每个元素,对 `post` 方法按**逆序**调用。`RenderSlice` 是规范实现;`RenderArray` 与 `RenderVec` 通过 `as_slice()` 委托给它。

## DefaultPassDispatcher —— 每个通道的基组件

**文件**: [platform/graphics/webgpu/src/frame/pass_base.rs](../../../../../rendiation/platform/graphics/webgpu/src/frame/pass_base.rs)
**构造函数**: `default_dispatcher(pass: &FrameRenderPass, reversed_depth: bool) -> DefaultPassDispatcher`

通常每个渲染通道都会把它作为第一个组件。它会:

- 绑定 `pass_info` 统一缓冲区(视口尺寸、纹素尺寸),使着色器可以查询 `ViewportRenderBufferSize` / `TexelSize`
- 注册渲染目标格式、深度模板状态与多重采样数
- 可选地自动向片元输出 0 写入白色(`auto_write: bool` —— 默认 `true`;当你自己写输出时设为 `false`,或使用便捷方法 `disable_auto_write()`)

在 `PassContent::render` 中的用法:

```rust
fn render(&mut self, pass: &mut FrameRenderPass) {
    let mut base = default_dispatcher(pass, false);
    base.auto_write = false;  // 我要自己写输出
    let components: [&dyn RenderComponent; 3] = [&base, &self.quad, &self.content];
    RenderArray(components).render(&mut pass.ctx, RenderMethod::TraditionalDraw(QUAD_DRAW_CMD));
}
```

## 综合示例

一个典型的可渲染组件:

```rust
// 定义着色器
impl GraphicsShaderProvider for MyEffect {
    fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
        builder.fragment(|builder, binding| {
            let uv = builder.query::<FragmentUv>();
            let tex = binding.bind_by(&self.input);
            let smp = binding.bind_by(&ImmediateGPUSamplerViewBind);
            builder.store_fragment_out_vec4f(0, tex.sample(smp, uv));
        });
    }
}

// 提供管线缓存键
impl ShaderHashProvider for MyEffect {
    shader_hash_type_id!();
}

// 在绘制时绑定资源
impl ShaderPassBuilder for MyEffect {
    fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
        ctx.binding.bind(&self.input);
        ctx.binding.bind_immediate_sampler(&TextureSampler::default().into_gpu());
    }
}

// RenderComponent 现已自动实现 —— MyEffect 可渲染了。
// 在通道中使用它(关于 pass() / .by() 的细节见 frame-pass-assemble skill):
pass("effect")
    .with_color(&target, store_full_frame())
    .render_ctx(ctx)
    .by(&mut MyEffect { input }.draw_quad());
```

### DrawCommand 变体

`DrawCommand` 控制顶点如何发出:

| 变体 | 用途 |
| --------- | ----- |
| `DrawCommand::Array { vertices, instances }` | 非索引绘制(全屏四边形使用:`0..4, 0..1`) |
| `DrawCommand::Indexed { indices, instances, base_vertex }` | 使用顶点/索引缓冲区的索引绘制 |
| `DrawCommand::Indirect { ... }` | GPU 驱动的间接绘制 |
| `DrawCommand::MultiIndirect { ... }` | 多次间接绘制 |
| `DrawCommand::MultiIndirectCount { ... }` | 带计数缓冲区的多次间接绘制 |
