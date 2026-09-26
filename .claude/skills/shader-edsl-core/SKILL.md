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

// Fixed-size local array
let arr: ShaderPtrOf<[f32; 16]> = make_local_var::<[f32; 16]>();
```

### Operators

The `std::ops` overloads follow the WGSL operator overload table, they do not reuse the math
library's operator impls (which contain WGSL-invalid ones like the homogeneous `Mat4 * Vec3`).

| Operator | Valid operands |
|----------|----------------|
| `+ -` | see `ShaderAddSub<Rhs>`: same type numeric scalar/vector, `vec ± scalar`, `scalar ± vec`, same type f32 matrix |
| `/ %` | see `ShaderDivRem<Rhs>`: same type numeric scalar/vector, `vec / scalar`, `scalar / vec` (same for `%`) |
| `*` | see `ShaderMul<Rhs>`: same type numeric scalar/vector, `vec * scalar`, `scalar * vec`, `mat * f32`, `f32 * mat`, `matCxR * vecC -> vecR`, `vecR * matCxR -> vecC`, `matCxR * matNxC -> matNxR` |
| unary `-` | i32/f32 scalar or vector (unsigned, bool and matrix are rejected) |
| `& \|` | integer or bool scalar/vector (non short circuit for bool) |
| `^ << >>`, `.bitwise_not()` | integer scalar/vector |
| `.and(v) .or(v)` | `Node<bool>`, short circuit |
| `.not()` | `Node<bool>` and `Node<VecN<bool>>` |

In the mixed vector and scalar form the scalar must be the vector component type, it applies to
every component, There is no need to `.splat()` the scalar operand manually. Matrix element type must be `f32`.

The compound assignments `+= -= *= /=` accept any right operand as long as the result type is the
left operand type, for example `v += s`, `v *= m` (vector times matrix), `m *= s`.

### Array types and indexing

| Array type | Indexed by | `.index(idx)` returns | Example |
|------------|-----------|----------------------|---------|
| `ShaderPtrOf<[T; N]>` | `Node<u32>` | `ShaderPtrOf<T>` | `arr.index(i).store(v)` / `.load()` |
| `ShaderReadonlyPtrOf<[T; N]>` | `Node<u32>` | `ShaderReadonlyPtrOf<T>` | `arr.index(i).load()` |
| `ShaderPtrOf<[T]>` (dynamic) | `Node<u32>` | `ShaderPtrOf<T>` | storage buffer rw |
| `ShaderReadonlyPtrOf<[T]>` (dynamic) | `Node<u32>` | `ShaderReadonlyPtrOf<T>` | storage buffer ro |

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
#[shader_struct]
#[derive(Clone, Copy, Debug)]
struct MyMaterial {
    pub base_color: Vec3<f32>,
    pub roughness: f32,
    pub metallic: f32,
}
```

`#[shader_struct]` auto-generates:

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
#[shader_struct(std140)] // must mark for uniform buffer
#[derive(Clone, Copy)]
struct MyUniform {
    pub color: Vec3<f32>,
    pub scale: f32,
}

#[repr(C)]
#[shader_struct(std430)] // must mark for storage buffer
#[derive(Clone, Copy)]
struct MyStorage {
    pub data: Vec4<f32>,
}
```

| Annotation | Alignment | Use case |
|------------|-----------|----------|
| `#[shader_struct(std140)]` | 16 bytes | Uniform Buffer |
| `#[shader_struct(std430)]` | Natural | Storage Buffer |
| `#[shader_struct]` | — | Shader-internal use (non-buffer) |

The macro must be placed before `#[derive(...)]`, because it inserts private padding fields for the host shareable struct.

### std140 specials

- "std140" here means the WGSL uniform address space layout, which is not GLSL std140
- `Mat2<f32>` is not supported in std140 (its WGSL uniform layout differs from the naga GLSL backend output), use `Vec4<f32>` instead
- host shareable matrices: std140 supports `Mat4`, `Mat2x4`, `Mat3x4` (and `Shader16PaddedMat3`); std430 additionally supports `Mat2`, `Mat3x2`, `Mat4x2`. The matCx3 types (`Mat3`, `Mat2x3`, `Mat4x3`) are not host shareable because the rust column stride is 12 bytes but 16 in WGSL
- **Bool**: cannot be used directly as a field in std140 and std430;  use `Bool` instead.
- use `Shader16PaddedMat3` instead of `Mat3<f32>` for std140-compatible mat3
- use `Shader140Array<T, N>` instead of `[T, N]` for std140-compatible fixed-size array; the element stride must be a multiple of 16 (`Vec4`, `Vec3`, structs, `Mat4` are fine; `f32`/`u32`/`Vec2` are rejected at compile time, pack them into `Vec4`)
- std430 fixed-size arrays and runtime arrays (`[T; N]`, `[T]`) can not use `Vec3` as element (rust stride 12 vs WGSL stride 16), rejected at compile time
- `#[shader_struct(std140)]` and `#[shader_struct(std430)]` require `#[repr(C)]` only (no `align`/`packed`), named public fields, no generics. Every field offset, the struct size and alignment are asserted at compile time, so a layout mismatch is always a compile error
- The rust layout is the source of truth: `#[shader_struct(std140)]`/`#[shader_struct(std430)]` attaches the rust field offsets to the struct meta info (`ShaderStructHostLayout`), the naga backend and the u32 serialization(std430) use them directly and inserts explicit padding members so the generated IR is always the natural WGSL layout (required by backends that ignore IR offsets, like WGSL text output for browser WebGPU and GLSL). Hand written host types should attach it by `ShaderStructMetaInfo::with_host_layout`


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
// Note: if have any else_if(), the ending must have else_by() or else_over() call.

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

WGSL has no labeled break, so `cx.do_break()` and `cx.do_continue()` panic at build time when `cx` is
not the nearest loop: calling the outer loop's ctx inside an inner loop, `do_break()` inside a
`switch_by` case (the WGSL break only exits the switch), or inside a function defined in the loop.
`do_continue()` inside a switch case is fine.

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

Convert counts, ranges and uniform/storage/local arrays into GPU-side iterables.

### Basic usage

```rust
// Counting loop
val(10_u32).into_shader_iter().for_each(|i, _| {
    // i: Node<u32>, from 0 to 9
});

// Iterate over storage buffer array, the array item is (index, pointer)
items.into_shader_iter().for_each(|(i, item), _| {
    let data = item.load();
    // ...
});
```

### Chained operations

```rust
samples
    .into_shader_iter()
    .clamp_by(sample_count.x())   // dynamically limit iteration count
    .map(|(i, sample)| {
        // i: Node<u32>, sample: ptr, the closure parameter types are inferred
        sample.load()
    })
    .sum()  // accumulate
```

### Supported adaptors

| Method | Purpose |
|--------|---------|
| `.map(f)` | Map |
| `.filter(pred)` | Filter, the item must be a right value (`Node<T>` or tuple of them) |
| `.filter_map(f)` | Filter + map, `f` returns `(Node<bool>, Node<O>)`, the input item must be `Node<T>` |
| `.zip(other)` | Zip two iterators |
| `.enumerate()` | With index |
| `.take_while(pred)` | Conditional truncation |
| `.clamp_by(count)` | Limit iteration count, the item must be `(Node<u32>, T)` indexed from 0 |
| `.flat_map(f)` | Flat map, `f` returns the inner iterator state, currently `ForRangeState` |
| `.for_each(f)` | Iterate, `f` gets the item and the `LoopCtx` of the iteration loop |
| `.sum()` | Sum |

The filtered item is stored in a variable to carry it out of the internal search loop, the pointer
item can not be stored, map it to the value first: `.map(|(i, p)| (i, p.load())).filter(..)`.

### Iteration sources

| Type | `into_shader_iter()` source |
|------|---------------------------|
| `u32` / `Node<u32>` | 0..n counting loop |
| `Node<Vec2<u32>>` | `ForRange`: from..to, empty if from >= to |
| StaticLengthArrayView | Compile-time known length array |
| DynLengthArrayView | Runtime-length array |

`ShaderStaticArrayReadonlyIter::from_array_clamp_length(array, len)` iterates the first `len`
items of a fixed size array (clamped by the array length).

### Iteration gotchas

- The iterator state is initialized where the iterator is created, create it right where the
  iteration starts. To pass the iterable around or iterate multiple times, pass the array view or
  anything implementing `IntoShaderIterator`. The stateful iterators are not `Clone` because the
  clone would share the same cursor variable.
- The `map` closure (and the `take_while` predicate) is also evaluated once for the final poll
  that ends the iteration, with the out of range index and the clamped array pointer. Do not put
  side effects (store, atomic) in them, put them in `for_each`.

## Texture Operations

### Sampling textures

```rust
// Basic sampling (implicit LOD)
let color: Node<Vec4<f32>> = texture.sample(sampler, uv);

// Zero-level sampling (no mipmap or explicit level 0), valid in any stage
let color = texture.sample_zero_level(sampler, uv);

// With explicit LOD, level is Node<f32> for float texture, Node<u32> for depth texture
let color = texture
    .build_sample_call(sampler, uv)
    .with_level(level)
    .sample();

// With LOD bias (float texture only, fragment stage only)
let color = texture
    .build_sample_call(sampler, uv)
    .with_level_bias(bias)
    .sample();

// With gradients (float texture only)
let color = texture
    .build_sample_call(sampler, uv)
    .with_level_grad(ddx, ddy)
    .sample();

// Offset (2D and 2D array only, each component in [-8, 7])
let color = texture
    .build_sample_call(sampler, uv)
    .with_offset(Vec2::new(1, -1))
    .with_zero_level()
    .sample();

// Gather (fetch four texels), also works for integer texture. Depth texture only allows channel X
let gathered = texture
    .build_sample_call(sampler, uv)
    .gather(GatherChannel::X);

// textureSampleBaseClampToEdge (ShaderTexture2D only)
let color = texture.sample_base_clamp_to_edge(sampler, uv);
```

- `sample`/`with_level`/`sample_zero_level` require `F: SamplerSampleTarget` (f32 and depth
  texture), integer textures can only be loaded or gathered.
- `with_level_bias`/`with_level_grad` require `F: SamplerBiasGradSampleTarget` (f32 texture).
- 1D texture coordinates are scalar (`Node<f32>` for sampling, `Node<u32>` for load).

### Depth comparison sampling

```rust
// requires a depth texture and ShaderCompareSampler
let visibility: Node<f32> = depth_texture
    .build_compare_sample_call(compare_sampler, uv, reference_depth)
    .sample(); // textureSampleCompareLevel, compare with base level by default

// textureSampleCompare (implicit level, fragment stage only)
let visibility = depth_texture
    .build_compare_sample_call(compare_sampler, uv, reference_depth)
    .with_implicit_level()
    .sample();

// textureGatherCompare, the four compare results
let results: Node<Vec4<f32>> = depth_texture
    .build_compare_sample_call(compare_sampler, uv, reference_depth)
    .gather();
```

### Direct load (sampler-less texel access)

```rust
// 2D texture, the level is required
let value = texture.load_texel(coord, level);

// 2D array
let value = texture.load_texel_layer(coord, layer, level);

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

Only the valid WGSL texture types implement `ShaderNodeType` (checked by `ValidShaderTextureType`),
so an invalid texture type can not be bound

### Texture metadata queries

```rust
let layers: Node<u32> = array_texture.texture_number_layers(); // array texture only
let levels: Node<u32> = texture.texture_number_levels();
let dims: Node<Vec2<u32>> = texture.texture_dimension_2d(None);  // None means base level

// storage texture has no mip level parameter
let dims: Node<Vec2<u32>> = storage_texture.texture_dimension_2d();
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

// Compare exchange, store 1 if current value is 0, may spuriously fail
let (old, exchanged): (Node<u32>, Node<bool>) = atomic_ptr.atomic_compare_exchange_weak(val(0), val(1));
```

Atomics are only valid in workgroup memory or read_write storage buffers.


## Subgroup Operations

Subgroup and quad operations are only valid in the fragment and compute stages. The arithmetic and
communication operations require numeric scalar/vector (bool is rejected).

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
let val: Node<f32> = value.subgroup_broadcast(3);        // broadcast to all, id: u32 in [0, 128)
let first: Node<f32> = value.subgroup_broadcast_first(); // from the lowest active invocation
let shuffled: Node<f32> = value.subgroup_shuffle(id);     // shuffle
let up: Node<f32> = value.subgroup_shuffle_up(delta);     // shuffle up
let down: Node<f32> = value.subgroup_shuffle_down(delta); // shuffle down
let xor: Node<f32> = value.subgroup_shuffle_xor(mask);    // shuffle xor
```

### Quad

```rust
let v: Node<f32> = value.quad_broadcast(1); // id: u32 in [0, 4)
let x: Node<f32> = value.quad_swap_x();
let y: Node<f32> = value.quad_swap_y();
let d: Node<f32> = value.quad_swap_diagonal();
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
- Body `let` bindings and fn parameters are automatically name marked for debugging in other graphics tools (same as `#[shader_name_marked]`), no extra attribute needed


## `#[shader_name_marked]` Debug Name Marking

Mark every `let xx = ...` binding in the function body with the Rust variable name, so the name
shows up as debug label in generated WGSL. Pure debugging aid, no runtime cost. Marking is best
effort: non-`Node<T>` values are no-ops, out-of-build-context calls are silently skipped, and a
node that is a global variable with an existing name keeps the original name.

```rust
#[shader_name_marked]
fn shade(hdr: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
    let exposure = hdr.saturate();
    let tonemapped = exposure.pow(val(2.2));
    tonemapped
}
```


## Built-in Math Functions

All methods are called directly on `Node<T>`.

### Arithmetic / Comparison

| Method | Description |
|--------|-------------|
| `.abs()` | Absolute value |
| `.min(v)` | Minimum |
| `.max(v)` | Maximum |
| `.clamp(low, high)` | Clamp |
| `.saturate()` | Clamp to [0, 1] (float only) |
| `.sign()` | Sign |
| `.step(edge)` | 1.0 if edge <= self, else 0.0 |
| `.smoothstep(low, high)` | Smooth step, scalar only |
| `.smoothstep_per_channel(low, high)` | Smooth step, low/high/self same type (scalar or vector) |
| `t.mix(a, b)` | Mix, t is f32 factor, a and b are the same scalar/vector type |
| `t.mix_per_channel(a, b)` | Mix, t/a/b same type |
| `.fma(b, c)` | `self * b + c` |
| `.degrees()` / `.radians()` | Angle conversion |
| `.equals(v)` | Equal |
| `.less_than(v)` | Less than |
| `.greater_than(v)` | Greater than |
| `.not_equals(v)` | Not equal |

### Vector

| Method | Description |
|--------|-------------|
| `.dot(v)` | Dot product (float or integer vector) |
| `.cross(v)` | Cross product (Vec3 only) |
| `.normalize()` | Normalize |
| `.length()` | Length |
| `.distance(v)` | Distance |
| `n.reflect(i)` | Reflect incident direction `i` by normal `n` |
| `n.refract(i, eta)` | Refract |
| `n.face_forward(i, n_ref)` | `n` if `dot(i, n_ref) < 0`, else `-n` |

### Matrix

| Method | Description |
|--------|-------------|
| `.transpose()` | Matrix transpose, `matCxR -> matRxC` |
| `.determinant()` | Determinant (square matrix only) |

### Math functions

`.sin()`, `.cos()`, `.tan()`, `.asin()`, `.acos()`, `.atan()`, `.atan2(other)`,
`.sinh()`, `.cosh()`, `.tanh()`,
`.exp()`, `.exp2()`, `.ln()` (log_e), `.log2()`,
`.pow(exp)`, `.sqrt()`, `.inverse_sqrt()`,
`.floor()`, `.ceil()`, `.round()`, `.fract()`, `.trunc()`,
`.frexp()` -> `(fract, exp)`, `.modf()` -> `(fract, whole)`, `.ldexp(exp)`, `.quantize_to_f16()`,
`.is_nan()`, `.is_inf()` (scalar f32, implemented by bit pattern check because WGSL has no such built-in)

### Integer bit functions

`.count_leading_zeros()`, `.count_trailing_zeros()`, `.count_one_bits()`, `.reverse_bits()`,
`.first_leading_bit()`, `.first_trailing_bit()`, `.extract_bits(offset, count)`,
`.insert_bits(new_bits, offset, count)`

### Packing

`pack4x8snorm`, `pack4x8unorm`, `pack2x16snorm`, `pack2x16unorm`, `pack2x16float` (float vector to u32),
`pack4x_i8`, `pack4x_i8_clamp` (`Vec4<i32>`), `pack4x_u8`, `pack4x_u8_clamp` (`Vec4<u32>`),
and the `unpack*` counterparts on `Node<u32>`, plus `dot4_u8_packed` / `dot4_i8_packed`

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
let b: Node<bool> = float_val.into_bool(); // u32/i32/f32 <-> bool are all supported
// 32 bit numeric (f32/u32/i32) scalar or vector, the shape must be the same
let bits: Node<u32> = float_val.bitcast::<u32>();
let bits: Node<Vec3<u32>> = float_vec3.bitcast::<Vec3<u32>>();
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

// From components (f32, u32, i32 and bool)
let v: Node<Vec4<f32>> = (val(1.0), val(2.0), val(3.0), val(1.0)).into();
```

### Swizzle

```rust
// Vector swizzle, the full WGSL set with both the xyzw and rgba names (can not be mixed)
let xy: Node<Vec2<f32>> = vec3.xy();
let zyx: Node<Vec3<f32>> = vec3.zyx();
let bgra: Node<Vec4<f32>> = vec4.bgra();
let xyxy: Node<Vec4<f32>> = vec2.xyxy();
let x: Node<f32> = vec4.x();

// By component indices, the out of range index is a compile error
let z: Node<f32> = vec4.component::<2>();
let wx: Node<Vec2<f32>> = vec4.swizzle2::<3, 0>();

// Splat (broadcast), works for any scalar type
let v4 = val(1.0).splat::<Vec4<f32>>();  // (1, 1, 1, 1)
```

All the swizzles of 1 to 4 components are generated for `Vec2`, `Vec3` and `Vec4` of any scalar
(by `impl_shader_swizzles!` in `rendiation-shader-derives`), they are `#[inline(always)]` wrappers
of `component` / `swizzle2` / `swizzle3` / `swizzle4`. The single component selection is an index
access, not a swizzle.

### Component references and swizzle assignment

```rust
let v: ShaderPtrOf<Vec4<f32>> = make_local_var();
v.x().store(val(1.));            // v.x = 1.0, the single component reference is ShaderPtrOf<f32>
v.index(i).store(val(1.));       // v[i] = 1.0
v.yz().store(val(Vec2::one()));  // v.yz = vec2(1.0), the swizzle assignment
let zyx = v.zyx().load();        // same as v.load().zyx()

let m: ShaderPtrOf<Mat4<f32>> = make_local_var();
m.w().store(val(Vec4::one()));   // m[3] = vec4(1.0), the column reference
m.index(i).x().store(val(1.));   // m[i].x = 1.0
```

- Component references exist on both `ShaderPtrOf` and `ShaderReadonlyPtrOf` vector/matrix
  pointers (and the u32 heap pointers of the combined buffer), `component::<I>()` is the index
  based one.
- The multi component swizzle view (`VectorSwizzlePtrView`) only exists on the writable pointer
  and requires distinct components (`v.xx()` is not generated, `swizzle2::<0, 0>()` is a compile
  error), for readonly or repeated components, load then swizzle.
- Like WGSL, the swizzle view read and write access the entire vector (the write is load, replace,
  store), so concurrently writing the different components of the same shared memory vector by
  swizzle view is a data race. Swizzle views can not be chained or indexed.

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

// Non square matrix, named MatCxR (C columns and R rows) like WGSL
let m: Node<Mat2x3<f32>> = (vec3_col0, vec3_col1).into();
let t: Node<Mat3x2<f32>> = m.transpose();
let v: Node<Vec3<f32>> = m * vec2_value; // matCxR * vecC -> vecR
```

All the WGSL matrix types are available: `Mat2`, `Mat3`, `Mat4`, `Mat2x3`, `Mat2x4`, `Mat3x2`,
`Mat3x4`, `Mat4x2`, `Mat4x3`, all of them can be composed from columns and support column access
(`.x()` `.y()` ...). `.determinant()` is square matrix only.


## Testing

EDSL tests live in `shader/api-testing`, see its README before adding tests.

When the result values matter (for example the iterator semantics), the test runs the shader on
the GPU by `gpu_map` of the `shader/api-testing` harness, and compares with the cpu reference logic.

## Gotchas

- No enum / sum types, Use `Node<bool>` flags + `.select()` / `.select_branched()`, or `switch_by`
- api relies on thread-local state, **do not call** across threads
