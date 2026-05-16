---
name: shader-edsl-binding-and-typed-container
description: >
  Covers rendiation's strongly-typed GPU resource containers (UniformBufferDataView,
  StorageBufferDataView, GPUTypedTextureView, GPUSamplerView, StorageTextureView) and how
  they bind on both the shader side (bind_by) and the pass side (bind). Use when creating
  GPU resources, wiring them into shaders, and binding them at render time.
  Depends on shader-edsl-core for the stage-agnostic language primitives.
metadata:
  version: "2.0"
  updated: "2026-05-16"
---

Typed GPU resource containers and the dual binding pipeline in rendiation. For the core language (types, expressions, control flow), see `shader-edsl-core`.

Key files:

| File | Purpose |
|------|---------|
| [platform/graphics/webgpu/src/resource/buffer/uniform.rs](platform/graphics/webgpu/src/resource/buffer/uniform.rs) | `UniformBufferDataView<T>`, `UniformBufferCachedDataView<T>` |
| [platform/graphics/webgpu/src/resource/buffer/storage.rs](platform/graphics/webgpu/src/resource/buffer/storage.rs) | `StorageBufferReadonlyDataView<T>`, `StorageBufferDataView<T>` |
| [platform/graphics/webgpu/src/resource/texture/mod.rs](platform/graphics/webgpu/src/resource/texture/mod.rs) | `GPUTypedTexture<D,F>`, `GPUTypedTextureView<D,F>` |
| [platform/graphics/webgpu/src/resource/texture/storage.rs](platform/graphics/webgpu/src/resource/texture/storage.rs) | `StorageTextureView<A,D,F>` |
| [platform/graphics/webgpu/src/resource/sampler.rs](platform/graphics/webgpu/src/resource/sampler.rs) | `GPUSamplerView`, `GPUComparisonSamplerView` |
| [platform/graphics/webgpu/src/pipeline/container.rs](platform/graphics/webgpu/src/pipeline/container.rs) | `ShaderBindingProvider` impls connecting containers to shader IR |


## Typed resource containers

rendiation wraps raw wgpu resources in strongly-typed containers that carry a Rust type parameter
mapping to the WGSL type. Each container binds on **two sides**:

- **Shader side** (`bind_by` in `ShaderBindGroupBuilder`) — produces a typed shader node (`ShaderReadonlyPtrOf<T>`, `BindingNode<ShaderTexture2D>`, etc.)
- **Pass side** (`bind` in `BindingBuilder`) — flushes the actual GPU resource into a bind group at draw time

The two sides must agree on bind group index and resource type — the same container is passed to both.

### UniformBufferDataView<T>

```rust
// T must implement Std140
pub struct UniformBufferDataView<T: Std140> {
    pub gpu: GPUBufferResourceView,
}

// Creation
let uniform = create_uniform(MyParams { ... }, &gpu);
let uniform = create_uniform_with_cache(MyParams { ... }, &gpu); // with CPU-side diff tracking
```

Shader-side binding:
```rust
let val: ShaderReadonlyPtrOf<MyParams> = builder.bind_by(&uniform);
let fields = val.load().expand();  // ENODE access
```

### StorageBufferReadonlyDataView<T> / StorageBufferDataView<T>

```rust
// T must implement Std430[MaybeUnsized]
pub struct StorageBufferReadonlyDataView<T: Std430MaybeUnsized + ?Sized> { pub gpu: GPUBufferResourceView; }
pub struct StorageBufferDataView<T: Std430MaybeUnsized + ?Sized> { pub gpu: GPUBufferResourceView; }

// Creation
let ro = create_gpu_readonly_storage(data.as_slice(), &gpu);        // [T], read-only
let rw = create_gpu_read_write_storage(StorageBufferInit::Zeroed(NonZeroU64::new(1024).unwrap()), &gpu);  // [T], read-write
```

Shader-side binding:
```rust
// Read-only
let input: ShaderReadonlyPtrOf<[MyItem]> = builder.bind_by(&ro);
let item = input.index(idx).load();

// Read-write
let output: ShaderPtrOf<[MyItem]> = builder.bind_by(&rw);
output.index(idx).store(value);

// Atomic access (via .into_device_atomic_array())
let atomic: ShaderAtomicPtrOf<[DeviceAtomic<u32>]> = rw.into_device_atomic_array();
atomic.index(idx).atomic_add(val(1));
```

### GPUTypedTextureView<D, F>

```rust
// D: texture dimension (TextureDimension2, TextureDimensionCube, etc.)
// F: format marker (f32, u32, TextureSampleDepth, MultiSampleOf<f32>, etc.)
pub struct GPUTypedTextureView<D, F> { pub gpu: GPUTextureView; }

// Common aliases
type GPU2DTextureView = GPUTypedTextureView<TextureDimension2, f32>;
type GPUCubeTextureView = GPUTypedTextureView<TextureDimensionCube, f32>;
type GPU2DDepthTextureView = GPUTypedTextureView<TextureDimension2, TextureSampleDepth>;
```

Shader-side binding — the produced type is determined by D and F, matching the container's type parameters:

```rust
// GPU2DTextureView → BindingNode<ShaderTexture<TextureDimension2, f32>>  (= BindingNode<ShaderTexture2D>)
let tex: BindingNode<ShaderTexture2D> = builder.bind_by(&diffuse);

// GPUCubeTextureView → BindingNode<ShaderTexture<TextureDimensionCube, f32>>  (= BindingNode<ShaderTextureCube>)
let cube: BindingNode<ShaderTextureCube> = builder.bind_by(&specular);

// GPU2DDepthTextureView → BindingNode<ShaderTexture<TextureDimension2, TextureSampleDepth>>
//   (= BindingNode<ShaderDepthTexture2D>)
let depth: BindingNode<ShaderDepthTexture2D> = builder.bind_by(&shadow_map);

// Storage texture → BindingNode<ShaderStorageTexture<A, D, F>>
let stor: BindingNode<ShaderStorageTextureRW2D> = builder.bind_by(&storage_view);
```

`ShaderTexture2D`, `ShaderTextureCube`, `ShaderDepthTexture2D`, `ShaderStorageTextureRW2D` etc.
are all type aliases for the generic `ShaderTexture<D, F>` or `ShaderStorageTexture<A, D, F>` with
specific dimension/format/access parameters.

### GPUSamplerView

```rust
pub type GPUSamplerView = ResourceViewRc<RawSampler>;

// Immediate default sampler (no explicit creation needed)
builder.bind_by(&ImmediateGPUSamplerViewBind);
```

### StorageTextureView<A, D, F>

```rust
// A: access mode (StorageTextureAccessReadWrite, Readonly, Writeonly)
let stor: BindingNode<ShaderStorageTextureRW2D> = builder.bind_by(&storage_view);
stor.write_texel(coord, value);
let val = stor.load_texel(coord);
```

### Dual binding example

```rust
// 1. Create the typed container
let uniform: UniformBufferDataView<Params> = create_uniform(params, &gpu.device);

// 2. Shader side (in GraphicsShaderProvider::build or ShaderComputePipelineBuilder)
let params_ptr: ShaderReadonlyPtrOf<Params> = builder.bind_by(&uniform);

// 3. Pass side (in ShaderPassBuilder::setup_pass or compute pass setup)
ctx.binding.bind(&uniform);
```

`bind_by` on the shader side and `bind` on the pass side must be called in the **same order**
for each container, matching bind group indices.


## Shader-side binding reference

`binding` is the second argument in `builder.fragment(|builder, binding| {})`, `builder.vertex(|builder, binding| {})`, or accessed via `ShaderComputePipelineBuilder` (which derefs to `ShaderBindGroupBuilder`).

```rust
// Texture
let tex: BindingNode<ShaderTexture2D> = binding.bind_by(&self.texture);

// Sampler (immediate, no container needed)
let sampler = binding.bind_by(&ImmediateGPUSamplerViewBind);

// Uniform buffer
let val: ShaderReadonlyPtrOf<MyUniform> = binding.bind_by(&self.uniform);

// Storage buffer (read-write)
let storage: ShaderPtrOf<[MyItem]> = binding.bind_by(&self.buffer);
storage.index(idx).store(value);
let item = storage.index(idx).load();

// Storage buffer (read-only)
let storage: ShaderReadonlyPtrOf<[MyItem]> = binding.bind_by(&self.buffer);
let item = storage.index(idx).load();

// Storage texture
let stor: BindingNode<ShaderStorageTextureRW2D> = binding.bind_by(&self.storage_texture);

// bind_single_by — simple binding, no BindingPreparer needed
let value = binding.bind_single_by(&self.config).load();
```

| Shader-side binding type | Produces |
|--------------------------|----------|
| `BindingNode<ShaderTexture2D>` | Texture binding node |
| `ImmediateGPUSamplerViewBind` | Default sampler |
| `ShaderReadonlyPtrOf<T>` | Read-only uniform/storage pointer |
| `ShaderPtrOf<T>` | Read-write storage pointer |
| `BindingNode<ShaderStorageTextureRW2D>` | Read-write storage texture |

### Non-filterable texture

```rust
let tex: BindingNode<ShaderTexture<TextureDimension2, DisableFiltering<f32>>> =
    binding.bind_by(&self.non_filter_tex);
```

### Cross-stage binding (vertex + fragment shared)

```rust
BindingPreparer::new(&src).using_graphics_pair(builder, register);
```

Used when a binding is needed in both vertex and fragment stages of the same graphics pipeline.

## Pass-side binding

In `ShaderPassBuilder::setup_pass`:

```rust
fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(&self.texture);
    ctx.binding.bind_immediate_sampler(&sampler_desc.into_gpu());
    ctx.binding.bind(&self.uniform);
    ctx.binding.bind(&self.storage);
}
```

`ctx.binding` is a `BindingBuilder`. The `.bind()` calls must follow the **same order** as the
shader-side `bind_by()` calls, since both determine bind group index assignment.
