---
name: shader-edsl-graphics
description: >
  rendiation 着色器 EDSL 的图形管线参考。涵盖 GraphicsShaderProvider、
  顶点/片元阶段、semantic(内置与自定义)、资源绑定(纹理、缓冲区、采样器)、
  渲染目标与常见图形配方。构建顶点+片元着色器管线时使用。
  阶段无关的语言原语依赖 shader-edsl-core,资源绑定依赖 shader-edsl-binding-and-typed-container。
metadata:
  version: "1.0"
  updated: "2026-05-16"
---

rendiation 图形管线参考。核心语言见 `shader-edsl-core`,资源绑定见 `shader-edsl-binding-and-typed-container`,计算管线见 `shader-edsl-compute`。

```rust
use rendiation_shader_api::*;
```


## 图形管线模板

```rust
impl GraphicsShaderProvider for MyPass {
    fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
        builder.fragment(|builder, binding| {
            // 绑定资源
            let tex: BindingNode<ShaderTexture2D> = binding.bind_by(&self.input_texture);
            let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);

            // 查询 semantic(依赖注入)
            let uv: Node<Vec2<f32>> = builder.query::<FragmentUv>();

            // 编写着色器逻辑
            let color = tex.sample(sampler, uv);

            // 输出
            builder.store_fragment_out(0, color);
        });
    }
}
```

**关键方法**:

- `builder.vertex(|builder, binding| { ... })` — 进入顶点阶段
- `builder.fragment(|builder, binding| { ... })` — 进入片元阶段
- `binding.bind_by(&resource)` — 绑定资源(纹理/缓冲区)
- `binding.bind_single_by(&resource)` — 绑定简单资源
- `builder.store_fragment_out(slot, value)` — 写入片元输出
- `builder.store_fragment_out_vec4f(slot, vec4)` — 将 vec4 写入 4xf32 输出(常用便捷方法)

### 完整的顶点 + 片元示例

```rust
fn build(&self, builder: &mut ShaderRenderPipelineBuilder) {
    builder.vertex(|builder, binding| {
        let pos = builder.query::<GeometryPosition>();
        let mvp = builder.query::<CameraViewNoneTranslationProjectionMatrix>();
        builder.set_vertex_out::<GeometryUV>(builder.query::<GeometryUV>());
        builder.register::<ClipPosition>(mvp * (pos, val(1.0)).into());
    });
    builder.fragment(|builder, binding| {
        let uv = builder.query_or_interpolate_by::<FragmentUv, GeometryUV>();
        let tex = binding.bind_by(&self.tex);
        let smp = binding.bind_by(&ImmediateGPUSamplerViewBind);
        builder.store_fragment_out_vec4f(0, tex.sample(smp, uv));
    });
}
```

### 注册顶点缓冲区(CPU 侧)

```rust
builder.register_vertex::<CommonVertex>(VertexStepMode::Vertex);
```


## 顶点输入与 Semantic

### 定义顶点输入布局

```rust
#[repr(C)]
#[derive(rendiation_shader_api::ShaderVertex, Clone, Copy, Debug)]
pub struct CommonVertex {
    #[semantic(GeometryPosition)]
    pub position: Vec3<f32>,
    #[semantic(GeometryNormal)]
    pub normal: Vec3<f32>,
    #[semantic(GeometryUV)]
    pub uv: Vec2<f32>,
}
```

`#[semantic(X)]` 将字段与内置 semantic 关联。`#[derive(ShaderVertex)]` 会生成 `ShaderVertexInProvider` 的实现。

### 查询顶点输入

```rust
let pos: Node<Vec3<f32>> = builder.query::<GeometryPosition>();
let normal: Node<Vec3<f32>> = builder.query::<GeometryNormal>();
```

### 设置顶点输出

```rust
builder.set_vertex_out::<GeometryUV>(uv);
builder.set_vertex_out_with_given_interpolate::<FragmentColor>(color);

// 内置顶点输出——必须写入 (x, y, z, w)
builder.register::<ClipPosition>(clip_pos);
```

### 自定义 semantic

```rust
only_vertex!(MyVertexData, Vec4<f32>);    // 仅顶点阶段
only_fragment!(MyFragData, Vec3<f32>);    // 仅片元阶段
both!(MySharedData, f32);                 // 顶点 + 片元共享
```

用法:

```rust
builder.query::<MyVertexData>();         // 读取
builder.register::<MySharedData>(val);   // 写入
```


## 片元着色器模式

### 核心方法

```rust
// 查询 semantic(未找到时 panic)
let uv: Node<Vec2<f32>> = builder.query::<FragmentUv>();

// 安全查询(返回 Option)
let color = builder.try_query::<FragmentColor>();

// 从片元查询,回退到顶点插值
let norm: Node<Vec3<f32>> = builder.query_or_interpolate_by::<FragmentRenderNormal, VertexRenderNormal>();

// 查询,或插入默认值
let val = builder.query_or_insert_default::<FragmentUv>();

// 注册一个 semantic 值
builder.register::<FragmentRenderNormal>(normal);

// 输出
builder.store_fragment_out(0, color);          // 输出到槽位 0
builder.store_fragment_out_vec4f(0, vec4);     // vec4 输出(常见)

// 多渲染目标
builder.define_out_by(channel(format));        // 声明新的输出槽位
builder.store_fragment_out(1, another_color);  // 写入槽位 1

// 特殊操作
builder.discard();                              // 丢弃片元
builder.register::<FragmentDepthOutput>(depth); // 写入深度

// 便捷方法
builder.get_or_compute_fragment_uv();           // 自动获取或计算 UV
builder.get_or_compute_fragment_normal();       // 自动获取或计算法线
```

### 片元输出模式

```rust
// 单输出
builder.store_fragment_out_vec4f(0, color);

// 多输出(例如延迟着色)
builder.define_out_by(channel(TextureFormat::Rgba8Unorm));      // 槽位 0
builder.define_out_by(channel(TextureFormat::Rgba16Float));     // 槽位 1
builder.store_fragment_out_vec4f(0, albedo);
builder.store_fragment_out_vec4f(1, normal_and_roughness);
```

## Semantic 速查表

### 顶点输入(几何数据,CPU 上传)

| Semantic | Rust 类型 |
|----------|-----------|
| `GeometryPosition` | `Vec3<f32>` |
| `GeometryPosition2D` | `Vec2<f32>` |
| `GeometryNormal` | `Vec3<f32>` |
| `GeometryTangent` | `Vec4<f32>` |
| `GeometryUV` (= `GeometryUVChannel<0>`) | `Vec2<f32>` |
| `GeometryUVChannel<I>` | `Vec2<f32>` |
| `GeometryColor` | `Vec3<f32>` |
| `GeometryColorWithAlpha` | `Vec4<f32>` |
| `JointIndexChannel<I>` | `Vec4<u32>` |
| `WeightChannel<I>` | `Vec4<f32>` |

### 顶点内置

| Semantic | 类型 | 描述 |
|----------|------|-------------|
| `VertexIndex` | `u32` | gl_VertexIndex |
| `VertexInstanceIndex` | `u32` | gl_InstanceIndex |

### 顶点输出

| Semantic | 类型 | 描述 |
|----------|------|-------------|
| `ClipPosition` | `Vec4<f32>` | 必须写入 (x, y, z, w) |
| `VertexRenderPosition` | `Vec3<f32>` | 世界空间位置 |
| `VertexRenderNormal` | `Vec3<f32>` | 世界空间法线 |

### 片元输入(由顶点输出插值而来)

| Semantic | 类型 | 描述 |
|----------|------|-------------|
| `FragmentFrontFacing` | `bool` | 面朝前 |
| `FragmentPosition` | `Vec4<f32>` | (x,y) = 帧缓冲坐标 |
| `FragmentSampleIndex` | `u32` | |
| `FragmentSampleMaskInput` | `u32` | |

### 片元共享(顶点写,片元读)

| Semantic | 类型 | 描述 |
|----------|------|-------------|
| `FragmentUv` | `Vec2<f32>` | 纹理坐标 |
| `FragmentRenderPosition` | `Vec3<f32>` | 世界空间位置 |
| `FragmentRenderNormal` | `Vec3<f32>` | 世界空间法线 |
| `FragmentColor` | `Vec3<f32>` | 顶点颜色 |

### 片元输出

| Semantic | 类型 | 描述 |
|----------|------|-------------|
| `FragmentDepthOutput` | `f32` | 深度写入 |
| `FragmentSampleMaskOutput` | `u32` | |

### 渲染上下文(自动提供)

| Semantic | 类型 | 描述 |
|----------|------|-------------|
| `ViewportRenderBufferSize` | `Vec2<f32>` | 视口分辨率 |
| `TexelSize` | `Vec2<f32>` | 1 / 分辨率 |
| `CameraProjectionMatrix` | `Mat4<f32>` | 投影矩阵 |
| `CameraProjectionInverseMatrix` | `Mat4<f32>` | 投影逆矩阵 |
| `CameraWorldNoneTranslationMatrix` | `Mat4<f32>` | 相机无平移矩阵 |
| `CameraWorldPositionHP` | `HighPrecisionTranslation` | 相机位置(高精度) |
| `CameraViewNoneTranslationProjectionMatrix` | `Mat4<f32>` | 视图-投影矩阵 |
| `CameraViewNoneTranslationProjectionInverseMatrix` | `Mat4<f32>` | 视图-投影逆矩阵 |
| `WorldPositionHP` | `HighPrecisionTranslation` | 物体世界位置(高精度) |
| `WorldNoneTranslationMatrix` | `Mat4<f32>` | 物体世界矩阵 |
| `WorldNormalMatrix` | `Mat3<f32>` | 法线矩阵 |

### 光照 / 渲染

| Semantic | 类型 | 描述 |
|----------|------|-------------|
| `ColorChannel` | `Vec3<f32>` | 基础色 |
| `EmissiveChannel` | `Vec3<f32>` | 自发光 |
| `AlphaChannel` | `f32` | Alpha |
| `HDRLightResult` | `Vec3<f32>` | HDR 光照结果(仅片元) |
| `LDRLightResult` | `Vec3<f32>` | LDR 光照结果(仅片元) |
| `ShouldUsePreSetLDRResult` | `bool` | 预设 LDR(仅片元) |
| `DefaultDisplay` | `Vec4<f32>` | 默认调试显示 |


## 常见模式(配方)

### 纹理绑定与采样

```rust
let tex: BindingNode<ShaderTexture2D> = binding.bind_by(&self.input);
let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);
let uv: Node<Vec2<f32>> = builder.query::<FragmentUv>();
let color = tex.sample(sampler, uv);
builder.store_fragment_out_vec4f(0, color);
```

### Uniform 结构体

```rust
let uniform: ShaderReadonlyPtrOf<Params> = binding.bind_by(&self.params);
let f = uniform.load().expand();
let value = f.field * val(2.0);
```

### 后处理通道(仅片元)

```rust
builder.fragment(|builder, binding| {
    let uv = builder.query::<FragmentUv>();
    let tex = binding.bind_by(&self.input);
    let smp = binding.bind_by(&ImmediateGPUSamplerViewBind);
    let color = tex.sample(smp, uv);
    builder.store_fragment_out_vec4f(0, color);
});
```

### 多渲染目标

```rust
builder.define_out_by(channel(TextureFormat::Rgba8Unorm));     // 槽位 0
builder.define_out_by(channel(TextureFormat::Rgba16Float));    // 槽位 1
builder.store_fragment_out_vec4f(0, albedo);
builder.store_fragment_out_vec4f(1, normal_roughness);
```

### SSAO 风格:迭代 + 累加

```rust
let result = samples
    .into_shader_iter()
    .clamp_by(sample_count)
    .map(|(_, sample): (_, ShaderReadonlyPtrOf<Vec4<f32>>)| {
        let s = sample.load();
        // 处理采样 ...
        val(0.0) // 返回贡献值
    })
    .sum();
```

### 动态数组迭代(例如模糊权重)

```rust
let weight_count: Node<u32> = binding.bind_by(&self.count).load().x();
let sum = weights
    .into_shader_iter()
    .clamp_by(weight_count)
    .map(|(i, weight): (_, ShaderReadonlyPtrOf<Vec4<f32>>)| {
        let w = weight.load();
        let sample_uv = uv + size * direction * i.into_f32();
        tex.sample(sampler, sample_uv) * w
    })
    .sum();
```


## 注意事项(图形专用)

### 片元输出

- 片元输出槽位必须在**使用前声明**(首次调用 `store_fragment_out` 时自动声明)
- 多个输出槽位需要显式调用 `define_out_by(channel(format))`

### 顶点 → 片元同步

- 顶点输出自动与片元输入同步(使用相同的 `both!` semantic)
- 使用 `builder.query_or_interpolate_by::<FragType, VertType>()` 声明依赖关系


## 参考示例

| 示例 | 文件 |
|---------|------|
| PBR 材质(GLES) | [scene/rendering/gpu-gles/src/material/mr.rs](../../../../../rendiation/scene/rendering/gpu-gles/src/material/mr.rs) |
| SSAO | [content/texture/gpu-process/src/ssao.rs](../../../../../rendiation/content/texture/gpu-process/src/ssao.rs) |
| FXAA | [content/texture/gpu-process/src/fxaa.rs](../../../../../rendiation/content/texture/gpu-process/src/fxaa.rs) |
| 线性模糊 | [content/texture/gpu-process/src/blur.rs](../../../../../rendiation/content/texture/gpu-process/src/blur.rs) |
| 色调映射 | [content/texture/gpu-process/src/tonemap.rs](../../../../../rendiation/content/texture/gpu-process/src/tonemap.rs) |
| 网格地面 | [application/viewer-content/src/rendering/grid_ground.rs](../../../../../rendiation/application/viewer-content/src/rendering/grid_ground.rs) |
| 宽线 | [extension/wide-line/src/draw.rs](../../../../../rendiation/extension/wide-line/src/draw.rs) |
| 字体渲染 | [extension/text-3d/src/slug_shader.rs](../../../../../rendiation/extension/text-3d/src/slug_shader.rs) |
