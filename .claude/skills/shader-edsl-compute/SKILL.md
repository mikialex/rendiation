---
name: shader-edsl-compute
description: >
  Compute pipeline reference for the rendiation shader EDSL. Covers ShaderComputePipelineBuilder,
  workgroup shared/private memory, barriers, built-in compute IDs, workgroup uniform load,
  ray tracing (wavefront compute backend), and compute-specific recipes like workgroup reduction.
  Use when building compute shader pipelines. Depends on shader-edsl-core for the
  stage-agnostic language primitives. For resource binding, see shader-edsl-binding-and-typed-container.
metadata:
  version: "1.0"
  updated: "2026-05-16"
---

Rendiation compute pipeline reference. For the core language see `shader-edsl-core`. For resource binding see `shader-edsl-binding-and-typed-container`. For graphics pipelines see `shader-edsl-graphics`.

```rust
use rendiation_shader_api::*;
```


## Compute Pipeline Template

```rust
pub fn create_compute_pipeline(
    info: Arc<GPUInfo>,
    resource: &MyResource,
) -> Result<ComputeShaderCompileResult, ShaderBuildError> {
    let mut builder = ShaderComputePipelineBuilder::new(info, ShaderRuntimeChecks::default());

    // Configure workgroup size
    builder.config_work_group_size((256, 1, 1));

    // Bind resources
    let input: BindingNode<ShaderStorageBufferRW<[MyData]>> =
        builder.bind_by(&resource.storage_buffer);
    let output: BindingNode<ShaderStorageBufferRW<[MyResult]>> =
        builder.bind_by(&resource.output_buffer);

    // Get built-in compute IDs
    let global_id = builder.global_invocation_id();
    let local_id = builder.local_invocation_id();

    // Define workgroup shared memory
    let shared: ShaderPtrOf<Vec4<f32>> =
        builder.define_workgroup_shared_var::<Vec4<f32>>();

    // Write to output buffer
    let idx = global_id.x();
    let data = input.index(idx).load();
    let result = /* ... */;
    output.index(idx).store(result);

    builder.build()
}
```


## Built-in Compute IDs

| Method | Return type |
|--------|-------------|
| `.global_invocation_id()` | `Node<Vec3<u32>>` |
| `.local_invocation_id()` | `Node<Vec3<u32>>` |
| `.local_invocation_index()` | `Node<u32>` |
| `.workgroup_id()` | `Node<Vec3<u32>>` |
| `.workgroup_count()` | `Node<Vec3<u32>>` |
| `.subgroup_invocation_id()` | `Node<u32>` (requires subgroup support) |
| `.subgroup_id()` | `Node<u32>` (requires subgroup support) |
| `.subgroup_size()` | `Node<u32>` (requires subgroup support) |

**Workgroup size config**: `IntoWorkgroupSize` trait, implemented for `u32`, `(u32, u32)`, `(u32, u32, u32)`


## Barriers

```rust
storage_barrier();     // storage memory barrier
workgroup_barrier();   // workgroup memory barrier
subgroup_barrier();    // subgroup barrier (requires SUBGROUP_BARRIER feature)
```


## Workgroup Shared & Private Memory

### Workgroup shared memory

```rust
// Fixed size
let shared: ShaderPtrOf<Vec4<f32>> = builder.define_workgroup_shared_var::<Vec4<f32>>();

// Host-specified size array (GPU sees fixed size, CPU side is dynamic)
let shared_arr: ShaderPtrOf<HostDynSizeArray<f32>> =
    builder.define_workgroup_shared_var_host_size_array::<f32>(len);
```

### Workgroup uniform load

```rust
// Broadcast a uniform value from workgroup memory to all invocations
let uniform_val: Node<f32> = workgroup_uniform_load(ptr);
```

## Common Patterns (Recipes)

### 7.1 Workgroup Reduction

```rust
let shared: ShaderPtrOf<f32> = builder.define_workgroup_shared_var_host_size_array::<f32>(256);
let lid = builder.local_invocation_id().x();

// Load into shared memory
shared.index(lid).store(data);
workgroup_barrier();

// Tree reduction
let mut step = val(128_u32);
loop_by(|cx| {
    if_by(lid.less_than(step), || {
        let a = shared.index(lid).load();
        let b = shared.index(lid + step).load();
        shared.index(lid).store(a + b);
    });
    step = step / val(2_u32);
    workgroup_barrier();
    if_by(step.equals(val(0_u32)), || { cx.do_break(); });
});

// Thread 0 holds the final result
let result = shared.index(val(0_u32)).load();
```

### 7.2 Counting loop over invocations

```rust
val(256_u32).into_shader_iter().for_each(|i, _| {
    // i: Node<u32>, 0..255
});
```

### 7.3 Iterate over storage buffer array

```rust
let buffer: BindingNode<ShaderStorageBufferRW<[Item]>> = builder.bind_by(&resource);
buffer.into_shader_iter().for_each(|item, _| {
    let data = item.load();
    // process data...
});
```

### 7.4 SSAO-style: iterate + accumulate

```rust
let result = samples
    .into_shader_iter()
    .clamp_by(sample_count)
    .map(|(_, sample): (_, ShaderReadonlyPtrOf<Vec4<f32>>)| {
        let s = sample.load();
        // process sample ...
        val(0.0)
    })
    .sum();
```

### 7.5 Subgroup prefix sum

```rust
let inclusive: Node<f32> = value.subgroup_inclusive_add();
let exclusive: Node<f32> = value.subgroup_exclusive_add();
```


## Reference Examples

| Example | File |
|---------|------|
| Compute 101 (prefix sum) | [platform/graphics/webgpu/examples/compute101.rs](platform/graphics/webgpu/examples/compute101.rs) |
| Ray tracing | [shader/ray-tracing/src/test.rs](shader/ray-tracing/src/test.rs) |
| Sampling library (`#[shader_fn]`) | [shader/library/src/sampling.rs](shader/library/src/sampling.rs) |
| Normal mapping | [shader/library/src/normal_mapping.rs](shader/library/src/normal_mapping.rs) |
