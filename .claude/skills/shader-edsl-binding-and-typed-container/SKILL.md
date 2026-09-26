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
| [platform/graphics/webgpu/src/binding/dynamic_offset.rs](platform/graphics/webgpu/src/binding/dynamic_offset.rs) | `DynamicOffsetBinding<T>`, `UniformBufferDynamicOffsetArray<T>` |
| [platform/graphics/webgpu/src/binding/declare.rs](platform/graphics/webgpu/src/binding/declare.rs) | `BindingDeclare`, `Binder`, `BindKind`: declare bindings once for both sides |


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
let uniform = create_uniform(MyParams { ... }, &gpu, "my_uniform");
let uniform = create_uniform_with_cache(MyParams { ... }, &gpu, "my_uniform"); // with CPU-side diff tracking
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
let ro = create_gpu_readonly_storage(data.as_slice(), &gpu, "my_ro_storage");        // [T], read-only
let rw = create_gpu_read_write_storage(StorageBufferInit::Zeroed(NonZeroU64::new(1024).unwrap()), &gpu, "my_rw_storage");  // [T], read-write
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
let atomic_view = rw.into_device_atomic_array();  // StorageBufferDataView<[DeviceAtomic<u32>]>
let atomic: ShaderPtrOf<[DeviceAtomic<u32>]> = builder.bind_by(&atomic_view);
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
// Create the typed container
let uniform: UniformBufferDataView<Params> = create_uniform(params, &gpu.device, "params_uniform");

// Shader side (in GraphicsShaderProvider::build or ShaderComputePipelineBuilder)
let params_ptr: ShaderReadonlyPtrOf<Params> = builder.bind_by(&uniform);

// Pass side (in ShaderPassBuilder::setup_pass or compute pass setup)
ctx.binding.bind(&uniform);
```

`bind_by` on the shader side and `bind` on the pass side must be called in the **same order**
for each container, matching bind group indices.


## Shader-side binding reference

`binding` is the second argument in `builder.fragment(|builder, binding| {})`, `builder.vertex(|builder, binding| {})`, or accessed via `ShaderComputePipelineBuilder::bindgroups()` (returns `&mut ShaderBindGroupBuilder`).

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

`DisableFiltering` wraps the binding provider (not the texture kind), it marks the texture binding
as unfilterable float and the sampler binding as non-filtering in the layout, the shader type is unchanged.

```rust
let tex: BindingNode<ShaderTexture2D> = binding.bind_by(&DisableFiltering(&self.depth));
let sampler = binding.bind_by(&DisableFiltering(ImmediateGPUSamplerViewBind));
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

## Dynamic offset buffer binding

`DynamicOffsetBinding<T>` wraps any uniform/storage buffer container (`UniformBufferDataView`,
`StorageBufferReadonlyDataView`, `StorageBufferDataView`, ...) and binds it with
`has_dynamic_offset: true` in the bindgroup layout. The inner view range decides the bindgroup
entry's base offset and size, `offset` is passed at `set_bind_group` time. The offset is **not**
part of the bindgroup cache key, so switching offset per draw reuses one bindgroup.

- Use the wrapper on **both** sides — layout differs from the non-dynamic binding. The offset is ignored on the shader side.
- The inner view should have an explicit size (`GPUBufferViewRange::size`), otherwise only offset 0 is valid.
- The offset must be aligned to `min_uniform_buffer_offset_alignment` / `min_storage_buffer_offset_alignment`; `dynamic_offset_stride(item_size, alignment)` helps.
- Not supported for binding arrays (asserted when creating the layout).
- Low level: `ShaderBindingDescriptor::has_dynamic_offset` on the shader side, `BindingBuilder::bind_dyn_with_dynamic_offset` on the pass side (call `check_binding_layout` first for custom sources).

`UniformBufferDynamicOffsetArray<T>` is a convenience container: one uniform buffer with `count`
aligned slots of `T`.

```rust
let per_draw = UniformBufferDynamicOffsetArray::<MyParams>::create(&gpu.device, count, "per_draw");
per_draw.write_at(&gpu.queue, i, &params);

// shader side, any index works
let params = binding.bind_by(&per_draw.bind_at(0)).load();

// pass side, per draw
ctx.binding.bind(&per_draw.bind_at(i));
```

## Declare bindings once (BindingDeclare)

The `bind_by` order in shader building must match the `bind` order in pass setup. Implement
`BindingDeclare` to write the binding list once; both sides run the same `declare`, so the order is
guaranteed by construction, and the shader side still gets fully typed instances. No macro needed.

- `BindKind` decides what each item produces: `ShaderKind` (shader instance), `GraphicsPairKind`
  (`GraphicsPairInputNodeAccessor`, use `.get()` in vertex/fragment), `PassKind` (`()`).
- `Binder` does the binding: `ShaderBinder` (inside a stage), `GraphicsPairBinder` (outside any
  stage, binds to both vertex and fragment; not for writeable storage), `PassBinder`.
- `with_group(idx, |b| ...)` assigns the bindgroup index on both sides, replacing `BindingController`.
- The result type depends only on the kind, not the binder, so it does not keep the builder borrowed.
- Optional bindings use `Option<K::Out<T>>`; nested components embed the child's `Bindings<K>`.
- Existing components keep working and can be mixed in the same pipeline.

```rust
pub struct MaterialBindings<K: BindKind> {
  pub color: K::Out<UniformBufferDataView<Vec4<f32>>>,
  pub tex: Option<(K::Out<GPU2DTextureView>, K::Out<GPUSamplerView>)>,
}

impl BindingDeclare for Material {
  type Bindings<K: BindKind> = MaterialBindings<K>;
  fn declare<B: Binder>(&self, b: &mut B) -> MaterialBindings<B::Kind> {
    MaterialBindings {
      color: b.bind(&self.color),
      tex: self.tex.as_ref().map(|t| (b.bind(t), b.bind(&self.sampler))),
    }
  }
}

// shader side, inside the fragment stage
builder.fragment(|builder, binding| {
  let b = self.declare(&mut ShaderBinder(binding));
  let color = b.color.load();
});

// pass side
fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
  self.declare(&mut PassBinder(&mut ctx.binding));
}
```

The full sample (nesting, `with_group`, `GraphicsPairBinder`, render verification) is the test
module in `binding/declare.rs`. Samplers can be declared as a cached `GPUSamplerView` from
`GPUDevice::create_and_cache_sampler_view`, so no immediate sampler wrapper is needed.
