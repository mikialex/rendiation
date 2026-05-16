---
name: shader-edsl-core
description: >
  Core language reference for the rendiation shader EDSL — stage-agnostic building blocks.
  Covers Node<T>, value construction, shader structs, memory layout, control flow, GPU-side
  iteration, texture operations, atomics, subgroups, #[shader_fn], math functions, and vector/matrix ops.
  Use when writing any shader expression, struct, or logic — regardless of pipeline stage.
metadata:
  version: "1.0"
  updated: "2026-05-16"
---

Rendiation uses a Rust-based EDSL (embedded domain-specific language) to generate WGSL-like shaders via the `naga` backend. This reference covers the **stage-agnostic core language** — types, expressions, control flow, and built-in functions. For pipeline integration (vertex/fragment/compute stages, semantics, binding), see `shader-edsl-graphics` and `shader-edsl-compute`.

```rust
use rendiation_shader_api::*;
```


## Core Concepts

### Node<T> — typed shader handle

`Node<T>` is the unified handle for all shader values. `Copy` + `Clone`. Math operations work through Rust's trait system.

```rust
// Create constants
let x: Node<f32> = val(1.0);
let v: Node<Vec3<f32>> = val(Vec3::one());

// Arithmetic (via std::ops overloads)
let sum = x + val(2.0);
let scaled = v * x;
let cmp: Node<bool> = x.less_than(val(3.0));

// Zero-initialized value
let zero: Node<Vec3<f32>> = zeroed_val();

// Mutable local variable
let slot: ShaderPtrOf<Vec3<f32>> = make_local_var::<Vec3<f32>>();
slot.store(val(Vec3::new(1.0, 0.0, 0.0)));
let loaded: Node<Vec3<f32>> = slot.load();

// Or initialize a local var from a Node value
let slot = val(Vec3::new(1.0, 0.0, 0.0)).make_local_var();
```

### Key type reference

| Type | Meaning |
|------|---------|
| `Node<T>` | Immutable shader value handle (Copy) |
| `ShaderPtrOf<T>` | Mutable pointer (supports store) |
| `ShaderReadonlyPtrOf<T>` | Read-only pointer (load only) |
| `ENode<T>` | Expanded struct fields (`<T as ShaderStructuralNodeType>::Instance`) |
| `BindingNode<T>` | Binding resource handle (`Node<ShaderBinding<T>>`) |

### ENode: struct field-level access

```rust
// Load from buffer and expand
let raw: Node<MyUniform> = buffer.load();
let fields: ENode<MyUniform> = raw.expand();

// Modify fields and reconstruct
let modified = ENode::<MyUniform> {
    roughness: fields.roughness * val(0.5),
    ..fields  // Rust struct update syntax
}.construct();
```


## Shader Structs & ENode

### Defining shader structs

```rust
#[repr(C)]
#[derive(Clone, Copy, Debug, ShaderStruct)]
struct MyMaterial {
    pub base_color: Vec3<f32>,
    pub roughness: f32,
    pub metallic: f32,
}
```

`#[derive(ShaderStruct)]` auto-generates:

```rust
struct MyMaterialShaderInstance {
    pub base_color: Node<Vec3<f32>>,
    pub roughness: Node<f32>,
    pub metallic: Node<f32>,
}
```

- `ENode<MyMaterial>` — alias to `MyMaterialShaderInstance`,the struct with all handles fields
- `MyMaterialShaderAPIInstance` — field accessors (`MyMaterial::base_color(node)`)
- `MyMaterialShaderAPIPtrInstance` / `MyMaterialShaderAPIReadonlyPtrInstance` — pointer views

### ENode expand and construct

```rust
// uniform buffer bind -> load -> expand
let mat: Node<MyMaterial> = binding.bind_by(&self.material).load();
let f = mat.expand();

// Use fields
let color = f.base_color;
let rough = f.roughness;

// Modify fields and reconstruct
let new_mat = ENode::<MyMaterial> {
    roughness: rough * val(0.5),
    ..f
}.construct();
```

### Access struct fields directly (without expand)

```rust
// Access field from Node<MyMaterial> (via generated ShaderAPIInstance)
let color: Node<Vec3<f32>> = MyMaterial::base_color(mat);
```


## Memory Layout Annotations

### std140 (Uniform Buffer) vs std430 (Storage Buffer)

```rust
#[repr(C)]
#[std140_layout]     // must mark for uniform buffer
#[derive(Clone, Copy, ShaderStruct)]
struct MyUniform {
    pub color: Vec3<f32>,
    pub scale: f32,
}

#[repr(C)]
#[std430_layout]     // must mark for storage buffer
#[derive(Clone, Copy, ShaderStruct)]
struct MyStorage {
    pub data: Vec4<f32>,
}
```

| Annotation | Alignment | Use case |
|------------|-----------|----------|
| `#[std140_layout]` | 16 bytes | Uniform Buffer |
| `#[std430_layout]` | Natural | Storage Buffer |
| None | — | Shader-internal use (non-buffer) |

### std140 specials

- **Bool**: cannot be used directly as a field in std140 and std430;  use `Bool` instead.
- use `Shader16PaddedMat3` instead of `Mat3<f32>` for std140-compatible mat3
- use `Shader140Array<T, N>` instead of `[T, N]` for std140-compatible fixed-size array


## Control Flow

### if_by / else_if / else_by

```rust
if_by(a.less_than(val(0.0)), || {
    // then
})
.else_if(a.greater_than(val(1.0)), || {
    // else if
})
.else_by(|| {
    // else
});
// Note: .else_if() and .else_by() can be skipped

```

### Ternary expression (branch-based select)

```rust
// Use select_branched — better than if_by for expression contexts
let result: Node<Vec3<f32>> = condition.select_branched(
    || val(Vec3::new(1.0, 0.0, 0.0)),   // true
    || val(Vec3::new(0.0, 0.0, 1.0)),   // false
);
```

### loop_by

```rust
loop_by(|cx| {
    // loop body
    if_by(should_stop, || {
        cx.do_break();
    });
    // Or skip iteration: cx.do_continue();
});
```

### switch_by

```rust
switch_by(selector)   // selector: Node<u32> or Node<i32>
    .case(0, || { /* ... */ })
    .case(1, || { /* ... */ })
    .end_with_default(|| { /* default */ });
    // Must call .end_with_default()!
```

### return

```rust
return_value(Some(value));  // return a value
do_return();               // return void
```

It's rare to use, only allowed in function ctx


## GPU-Side Iteration (`into_shader_iter`)

Convert uniform/storage buffer arrays into GPU-side iterables.

### Basic usage

```rust
// Counting loop
val(10_u32).into_shader_iter().for_each(|i, _| {
    // i: Node<u32>, from 0 to 9
});

// Iterate over storage buffer array
items.into_shader_iter().for_each(|item, _| {
    let data = item.load();
    // ...
});
```

### Chained operations

```rust
samples
    .into_shader_iter()
    .clamp_by(sample_count.x())   // dynamically limit iteration count
    .map(|(i, sample): (_, ShaderReadonlyPtrOf<Vec4<f32>>)| {
        // i: Node<u32>, sample: ptr
        sample.load()
    })
    .sum()  // accumulate
```

### Supported adaptors

| Method | Purpose |
|--------|---------|
| `.map(f)` | Map |
| `.filter(pred)` | Filter |
| `.filter_map(f)` | Filter + map |
| `.zip(other)` | Zip two iterators |
| `.enumerate()` | With index |
| `.take_while(pred)` | Conditional truncation |
| `.clamp_by(count)` | Limit iteration count |
| `.flat_map(f)` | Flat map |
| `.for_each(f)` | Iterate |
| `.sum()` | Sum |

### Iteration sources

| Type | `into_shader_iter()` source |
|------|---------------------------|
| `u32` / `Node<u32>` | 0..n counting loop |
| `Node<Vec2<u32>>` | `ForRange`: from..to |
| StaticLengthArrayView | Compile-time known length array |
| DynLengthArrayView | Runtime-length array |


## Texture Operations

### Sampling textures

```rust
// Basic sampling (implicit LOD)
let color: Node<Vec4<f32>> = texture.sample(sampler, uv);

// Zero-level sampling (no mipmap or explicit level 0)
let color = texture.sample_zero_level(sampler, uv);

// With explicit LOD
let color = texture
    .build_sample_call(sampler, uv)
    .with_level(level)
    .sample();

// With LOD bias
let color = texture
    .build_sample_call(sampler, uv)
    .with_level_bias(bias)
    .sample();

// With gradients
let color = texture
    .build_sample_call(sampler, uv)
    .with_level_grad(ddx, ddy)
    .sample();

// Gather (fetch four texels)
let gathered = texture
    .build_sample_call(sampler, uv)
    .gather(GatherChannel::Red);
```

### Direct load (sampler-less texel access)

```rust
// 2D texture
let value = texture.load_texel(coord);

// 2D array
let value = texture.load_texel_layer(coord, layer);

// Multisample
let value = texture.load_texel_multi_sample_index(coord, sample_index);
```

### Storage Texture (read-write)

```rust
// Read
let value = storage_tex.load_texel(coord);

// Write
storage_tex.write_texel(coord, value);
storage_tex.write_texel_index(coord, index, value); // array layer
```

### Texture type aliases

| Alias | Full type |
|-------|-----------|
| `ShaderTexture2D` | `ShaderTexture<TextureDimension2, f32>` |
| `ShaderTexture3D` | `ShaderTexture<TextureDimension3, f32>` |
| `ShaderTextureCube` | `ShaderTexture<TextureDimensionCube, f32>` |
| `ShaderTexture2DArray` | `ShaderTexture<TextureDimension2Array, f32>` |
| `ShaderDepthTexture2D` | `ShaderTexture<TextureDimension2, TextureSampleDepth>` |
| `ShaderMultiSampleTexture2D` | `ShaderTexture<TextureDimension2, MultiSampleOf<f32>>` |
| `ShaderStorageTextureRW2D` | `ShaderStorageTexture<StorageTextureAccessReadWrite, TextureDimension2, f32>` |
| `ShaderStorageTextureR2D` | `ShaderStorageTexture<StorageTextureAccessReadonly, TextureDimension2, f32>` |
| `ShaderStorageTextureW2D` | `ShaderStorageTexture<StorageTextureAccessWriteonly, TextureDimension2, f32>` |

### Texture metadata queries

```rust
let layers: Node<u32> = texture.texture_number_layers();
let levels: Node<u32> = texture.texture_number_levels();
let dims: Node<Vec2<u32>> = texture.texture_dimension_2d();
```


## Atomic Operations

```rust
// Pointer view for atomic types
let atomic_ptr: ShaderPtrOf<DeviceAtomic<T>> = /* from buffer or shared mem */;

// Basic atomic ops
let old: Node<u32> = atomic_ptr.atomic_load();
atomic_ptr.atomic_store(val(42));
let old: Node<u32> = atomic_ptr.atomic_exchange(val(0));

// Arithmetic atomic ops
let old = atomic_ptr.atomic_add(val(1));
let old = atomic_ptr.atomic_sub(val(1));
let old = atomic_ptr.atomic_min(val(10));
let old = atomic_ptr.atomic_max(val(100));

// Bitwise atomic ops
let old = atomic_ptr.atomic_and(val(0xFF));
let old = atomic_ptr.atomic_or(val(0x01));
let old = atomic_ptr.atomic_xor(val(0xFF));

```


## Subgroup Operations

### Collective Reduce

```rust
let sum: Node<f32> = value.subgroup_add();
let product: Node<f32> = value.subgroup_mul();
let min: Node<f32> = value.subgroup_min();
let max: Node<f32> = value.subgroup_max();
```

### Scan

```rust
let inclusive: Node<f32> = value.subgroup_inclusive_add();
let exclusive: Node<f32> = value.subgroup_exclusive_add();
let excl_mul: Node<f32> = value.subgroup_exclusive_mul();
let incl_mul: Node<f32> = value.subgroup_inclusive_mul();
```

### Communication

```rust
let val: Node<f32> = value.subgroup_broadcast(id);       // broadcast to all
let shuffled: Node<f32> = value.subgroup_shuffle(id);     // shuffle
let up: Node<f32> = value.subgroup_shuffle_up(delta);     // shuffle up
let down: Node<f32> = value.subgroup_shuffle_down(delta); // shuffle down
```

### Boolean

```rust
let all: Node<bool> = condition.subgroup_all();            // all true
let any: Node<bool> = condition.subgroup_any();            // any true
let ballot: Node<Vec4<u32>> = condition.subgroup_ballot(); // bitmask
```

### Integer Bitwise

```rust
let and: Node<u32> = value.subgroup_and();
let or: Node<u32> = value.subgroup_or();
let xor: Node<u32> = value.subgroup_xor();
```


## `#[shader_fn]` Reusable Functions

Define reusable functions callable on the GPU, auto-deduplicated.

```rust
#[shader_fn]
fn my_mix(a: Node<Vec3<f32>>, b: Node<Vec3<f32>>, t: Node<f32>) -> Node<Vec3<f32>> {
    a * (val(1.0) - t) + b * t
}

// GPU-side call:
let result = my_mix_fn(color1, color2, factor);
```

**Rules**:

- Parameters must be `Node<T>` types
- Return type is auto-inferred (annotation optional)
- Can call other `#[shader_fn]` functions
- Can use control flow internally (`if_by`, `loop_by`, etc.)
- Called with `_fn` suffix (generated by the macro)


## Built-in Math Functions

All methods are called directly on `Node<T>`.

### Arithmetic / Comparison

| Method | Description |
|--------|-------------|
| `.abs()` | Absolute value |
| `.min(v)` | Minimum |
| `.max(v)` | Maximum |
| `.clamp(low, high)` | Clamp |
| `.saturate()` | Clamp to [0, 1] |
| `.sign()` | Sign |
| `.step(edge)` | Step |
| `.smoothstep(low, high)` | Smooth step |
| `.mix(a, b, t)` | Mix (a, b same Node type, t is factor) |
| `.equals(v)` | Equal |
| `.less_than(v)` | Less than |
| `.greater_than(v)` | Greater than |
| `.not_equals(v)` | Not equal |

### Vector

| Method | Description |
|--------|-------------|
| `.dot(v)` | Dot product |
| `.cross(v)` | Cross product (Vec3 only) |
| `.normalize()` | Normalize |
| `.length()` | Length |
| `.distance(v)` | Distance |
| `.reflect(n)` | Reflect |
| `.refract(n, eta)` | Refract |

### Matrix

| Method | Description |
|--------|-------------|
| `.transpose()` | Matrix transpose |

### Math functions

`.sin()`, `.cos()`, `.tan()`, `.asin()`, `.acos()`, `.atan()`, `.atan2(other)`,
`.sinh()`, `.cosh()`, `.tanh()`,
`.exp()`, `.exp2()`, `.ln()` (log_e), `.log2()`,
`.pow(exp)`, `.sqrt()`, `.inverse_sqrt()`,
`.floor()`, `.ceil()`, `.round()`, `.fract()`, `.trunc()`

### Boolean / Selection

| Expression | Description |
|------------|-------------|
| `x.select(true_val, false_val)` | Conditional selection |
| `x.all()` | `Node<Vec<bool>> -> Node<bool>` — all true |
| `x.any()` | `Node<Vec<bool>> -> Node<bool>` — any true |
| `x.and(y)` | Logical AND |
| `x.or(y)` | Logical OR |
| `x.not()` | Logical NOT |

### Screen-space derivatives

```rust
let dx: Node<Vec3<f32>> = value.dpdx();   // dFdx
let dy: Node<Vec3<f32>> = value.dpdy();   // dFdy
let w: Node<Vec3<f32>> = value.fwidth();  // fwidth
```

### Type conversions

```rust
let f: Node<f32> = int_val.into_f32();
let u: Node<u32> = float_val.into_u32();
let i: Node<i32> = float_val.into_i32();
let bits: Node<u32> = float_val.bitcast::<u32>();
```

### Vector boolean operations

```rust
// Per-component select
let result = mask.select(if_true, if_false);
// mask: Node<VecN<bool>>, if_true/if_false: VecN<T>
```


## Vector and Matrix Construction

### Vector construction

```rust
// From scalars
let v3: Node<Vec3<f32>> = val(Vec3::new(1.0, 2.0, 3.0));

// From components
let v: Node<Vec4<f32>> = (val(1.0), val(2.0), val(3.0), val(1.0)).into();
```

### Swizzle

```rust
// Vector swizzle (x/y/z/w components)
let xy: Node<Vec2<f32>> = vec3.xy();
let xyz: Node<Vec3<f32>> = vec4.xyz();
let yx: Node<Vec2<f32>> = vec2.yx();
let zyx: Node<Vec3<f32>> = vec3.zyx();
let x: Node<f32> = vec4.x();

// Color channels
let rgb: Node<Vec3<f32>> = vec4.rgb();
let a: Node<f32> = vec4.a();

// Splat (broadcast)
let v4 = val(1.0).splat::<Vec4<f32>>();  // (1, 1, 1, 1)
```

### Matrix construction

```rust
// From 3 column vectors
let m: Node<Mat3<f32>> = (col0, col1, col2).into();

// From 4 column vectors
let m: Node<Mat4<f32>> = (col0, col1, col2, col3).into();

// Matrix access
let col: Node<Vec4<f32>> = mat.x();
let pos: Node<Vec3<f32>> = mat.position();   // mat4 last column(position)
let fwd: Node<Vec3<f32>> = mat.forward();    // mat4 3rd column (z)
let rot: Node<Mat3<f32>> = mat.shrink_to_3(); // mat4 -> mat3
```


## Gotchas

- No enum / sum types, Use `Node<bool>` flags + `.select()` / `.select_branched()`, or `switch_by`
- api relies on thread-local state, **do not call** across threads
