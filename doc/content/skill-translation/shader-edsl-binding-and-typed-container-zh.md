---
name: shader-edsl-binding-and-typed-container
description: >
  rendiation 强类型 GPU 资源容器(UniformBufferDataView、
  StorageBufferDataView、GPUTypedTextureView、GPUSamplerView、StorageTextureView)
  的参考文档,以及它们在着色器侧(bind_by)与通道侧(bind)的双向绑定方式。
  在创建 GPU 资源、将其接入着色器并在渲染时绑定时使用。
  依赖 shader-edsl-core 提供与阶段无关的语言原语。
metadata:
  version: "2.0"
  updated: "2026-05-16"
---

rendiation 的类型化 GPU 资源容器与双向绑定管线。核心语言(类型、表达式、控制流)参见 `shader-edsl-core`。

关键文件:

| 文件 | 用途 |
|------|---------|
| [platform/graphics/webgpu/src/resource/buffer/uniform.rs](../../../../../rendiation/platform/graphics/webgpu/src/resource/buffer/uniform.rs) | `UniformBufferDataView<T>`、`UniformBufferCachedDataView<T>` |
| [platform/graphics/webgpu/src/resource/buffer/storage.rs](../../../../../rendiation/platform/graphics/webgpu/src/resource/buffer/storage.rs) | `StorageBufferReadonlyDataView<T>`、`StorageBufferDataView<T>` |
| [platform/graphics/webgpu/src/resource/texture/mod.rs](../../../../../rendiation/platform/graphics/webgpu/src/resource/texture/mod.rs) | `GPUTypedTexture<D,F>`、`GPUTypedTextureView<D,F>` |
| [platform/graphics/webgpu/src/resource/texture/storage.rs](../../../../../rendiation/platform/graphics/webgpu/src/resource/texture/storage.rs) | `StorageTextureView<A,D,F>` |
| [platform/graphics/webgpu/src/resource/sampler.rs](../../../../../rendiation/platform/graphics/webgpu/src/resource/sampler.rs) | `GPUSamplerView`、`GPUComparisonSamplerView` |
| [platform/graphics/webgpu/src/pipeline/container.rs](../../../../../rendiation/platform/graphics/webgpu/src/pipeline/container.rs) | 将容器接入着色器 IR 的 `ShaderBindingProvider` 实现 |


## 类型化资源容器

rendiation 将原始 wgpu 资源封装进携带 Rust 类型参数(映射到 WGSL 类型)的强类型容器中。每个容器在**两侧**绑定:

- **着色器侧**(`ShaderBindGroupBuilder` 中的 `bind_by`)——产生类型化的着色器节点(`ShaderReadonlyPtrOf<T>`、`BindingNode<ShaderTexture2D>` 等)
- **通道侧**(`BindingBuilder` 中的 `bind`)——在绘制时把实际 GPU 资源写入绑定组

两侧必须在绑定组索引与资源类型上保持一致——同一个容器会同时传给两侧。

### UniformBufferDataView<T>

```rust
// T 必须实现 Std140
pub struct UniformBufferDataView<T: Std140> {
    pub gpu: GPUBufferResourceView,
}

// 创建
let uniform = create_uniform(MyParams { ... }, &gpu, "my_uniform");
let uniform = create_uniform_with_cache(MyParams { ... }, &gpu, "my_uniform"); // 带 CPU 侧差异跟踪
```

着色器侧绑定:
```rust
let val: ShaderReadonlyPtrOf<MyParams> = builder.bind_by(&uniform);
let fields = val.load().expand();  // ENode 访问
```

### StorageBufferReadonlyDataView<T> / StorageBufferDataView<T>

```rust
// T 必须实现 Std430MaybeUnsized
pub struct StorageBufferReadonlyDataView<T: Std430MaybeUnsized + ?Sized> { pub gpu: GPUBufferResourceView; }
pub struct StorageBufferDataView<T: Std430MaybeUnsized + ?Sized> { pub gpu: GPUBufferResourceView; }

// 创建
let ro = create_gpu_readonly_storage(data.as_slice(), &gpu, "my_ro_storage");        // [T],只读
let rw = create_gpu_read_write_storage(StorageBufferInit::Zeroed(NonZeroU64::new(1024).unwrap()), &gpu, "my_rw_storage");  // [T],读写
```

着色器侧绑定:
```rust
// 只读
let input: ShaderReadonlyPtrOf<[MyItem]> = builder.bind_by(&ro);
let item = input.index(idx).load();

// 读写
let output: ShaderPtrOf<[MyItem]> = builder.bind_by(&rw);
output.index(idx).store(value);

// 原子访问(经由 .into_device_atomic_array())
let atomic_view = rw.into_device_atomic_array();  // StorageBufferDataView<[DeviceAtomic<u32>]>
let atomic: ShaderPtrOf<[DeviceAtomic<u32>]> = builder.bind_by(&atomic_view);
atomic.index(idx).atomic_add(val(1));
```

### GPUTypedTextureView<D, F>

```rust
// D:纹理维度(TextureDimension2、TextureDimensionCube 等)
// F:格式标记(f32、u32、TextureSampleDepth、MultiSampleOf<f32> 等)
pub struct GPUTypedTextureView<D, F> { pub gpu: GPUTextureView; }

// 常用别名
type GPU2DTextureView = GPUTypedTextureView<TextureDimension2, f32>;
type GPUCubeTextureView = GPUTypedTextureView<TextureDimensionCube, f32>;
type GPU2DDepthTextureView = GPUTypedTextureView<TextureDimension2, TextureSampleDepth>;
```

着色器侧绑定——产生类型由 D 与 F 决定,与容器的类型参数一致:

```rust
// GPU2DTextureView → BindingNode<ShaderTexture<TextureDimension2, f32>>  (= BindingNode<ShaderTexture2D>)
let tex: BindingNode<ShaderTexture2D> = builder.bind_by(&diffuse);

// GPUCubeTextureView → BindingNode<ShaderTexture<TextureDimensionCube, f32>>  (= BindingNode<ShaderTextureCube>)
let cube: BindingNode<ShaderTextureCube> = builder.bind_by(&specular);

// GPU2DDepthTextureView → BindingNode<ShaderTexture<TextureDimension2, TextureSampleDepth>>
//   (= BindingNode<ShaderDepthTexture2D>)
let depth: BindingNode<ShaderDepthTexture2D> = builder.bind_by(&shadow_map);

// 存储纹理 → BindingNode<ShaderStorageTexture<A, D, F>>
let stor: BindingNode<ShaderStorageTextureRW2D> = builder.bind_by(&storage_view);
```

`ShaderTexture2D`、`ShaderTextureCube`、`ShaderDepthTexture2D`、`ShaderStorageTextureRW2D` 等
都是泛型 `ShaderTexture<D, F>` 或 `ShaderStorageTexture<A, D, F>` 在特定维度/格式/访问参数下的类型别名。

### GPUSamplerView

```rust
pub type GPUSamplerView = ResourceViewRc<RawSampler>;

// 直接使用默认采样器(无需显式创建)
builder.bind_by(&ImmediateGPUSamplerViewBind);
```

### StorageTextureView<A, D, F>

```rust
// A:访问模式(StorageTextureAccessReadWrite、Readonly、Writeonly)
let stor: BindingNode<ShaderStorageTextureRW2D> = builder.bind_by(&storage_view);
stor.write_texel(coord, value);
let val = stor.load_texel(coord);
```

### 双向绑定示例

```rust
// 创建类型化容器
let uniform: UniformBufferDataView<Params> = create_uniform(params, &gpu.device, "params_uniform");

// 着色器侧(在 GraphicsShaderProvider::build 或 ShaderComputePipelineBuilder 中)
let params_ptr: ShaderReadonlyPtrOf<Params> = builder.bind_by(&uniform);

// 通道侧(在 ShaderPassBuilder::setup_pass 或计算通道设置中)
ctx.binding.bind(&uniform);
```

着色器侧的 `bind_by` 与通道侧的 `bind` 必须为每个容器按**相同顺序**调用,以匹配绑定组索引。


## 着色器侧绑定参考

`binding` 是 `builder.fragment(|builder, binding| {})`、`builder.vertex(|builder, binding| {})` 中的第二个参数,或通过 `ShaderComputePipelineBuilder::bindgroups()` 方法取得(`&mut ShaderBindGroupBuilder`)。

```rust
// 纹理
let tex: BindingNode<ShaderTexture2D> = binding.bind_by(&self.texture);

// 采样器(直接使用,无需容器)
let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);

// 统一缓冲区
let val: ShaderReadonlyPtrOf<MyUniform> = binding.bind_by(&self.uniform);

// 存储缓冲区(读写)
let storage: ShaderPtrOf<[MyItem]> = binding.bind_by(&self.buffer);
storage.index(idx).store(value);
let item = storage.index(idx).load();

// 存储缓冲区(只读)
let storage: ShaderReadonlyPtrOf<[MyItem]> = binding.bind_by(&self.buffer);
let item = storage.index(idx).load();

// 存储纹理
let stor: BindingNode<ShaderStorageTextureRW2D> = binding.bind_by(&self.storage_texture);

// bind_single_by——简单绑定,无需 BindingPreparer
let value = binding.bind_single_by(&self.config).load();
```

| 着色器侧绑定类型 | 产生 |
|--------------------------|----------|
| `BindingNode<ShaderTexture2D>` | 纹理绑定节点 |
| `ImmediateGPUSamplerViewBind` | 默认采样器 |
| `ShaderReadonlyPtrOf<T>` | 只读统一/存储指针 |
| `ShaderPtrOf<T>` | 读写存储指针 |
| `BindingNode<ShaderStorageTextureRW2D>` | 读写存储纹理 |

### 不可过滤纹理

```rust
let tex: BindingNode<ShaderTexture<TextureDimension2, DisableFiltering<f32>>> =
    binding.bind_by(&self.non_filter_tex);
```

### 跨阶段绑定(顶点与片元共享)

```rust
BindingPreparer::new(&src).using_graphics_pair(builder, register);
```

当同一个图形管线的顶点与片元阶段都需要某个绑定时使用。

## 通道侧绑定

在 `ShaderPassBuilder::setup_pass` 中:

```rust
fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.texture);
    ctx.binding.bind_immediate_sampler(&sampler_desc.into_gpu());
    ctx.binding.bind(&self.uniform);
    ctx.binding.bind(&self.storage);
}
```

`ctx.binding` 是 `BindingBuilder`。`.bind()` 调用必须与着色器侧的 `bind_by()` 调用保持**相同顺序**,因为两者共同决定绑定组索引的分配。
