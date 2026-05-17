---
name: shader-edsl-compute
description: >
  Compute pipeline reference for the rendiation shader EDSL. Covers pipeline building , GPU unit testing, workgroup shared/private memory, barriers,
  built-in compute IDs, workgroup uniform load, ray tracing (wavefront compute backend),
  and compute-specific recipes like workgroup reduction.
  Use when building compute shader pipelines or writing GPU unit tests.
  Depends on shader-edsl-core for the stage-agnostic language primitives.
  Depends on  shader-edsl-binding-and-typed-container.
metadata:
  version: "1.1"
  updated: "2026-05-17"
---

Rendiation compute pipeline reference. For the core language see `shader-edsl-core`. For resource binding see `shader-edsl-binding-and-typed-container`. For graphics pipelines see `shader-edsl-graphics`.

```rust
use rendiation_shader_api::*;
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

### Workgroup Reduction

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

### Counting loop over invocations

```rust
val(256_u32).into_shader_iter().for_each(|i, _| {
    // i: Node<u32>, 0..255
});
```

### Iterate over storage buffer array

```rust
let buffer: BindingNode<ShaderStorageBufferRW<[Item]>> = builder.bind_by(&resource);
buffer.into_shader_iter().for_each(|item, _| {
    let data = item.load();
    // process data...
});
```

### SSAO-style: iterate + accumulate

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

### Subgroup prefix sum

```rust
let inclusive: Node<f32> = value.subgroup_inclusive_add();
let exclusive: Node<f32> = value.subgroup_exclusive_add();
```


## Compute Pipeline Template

```rust
pub fn build_my_pipeline(gpu: &GPU, ...) -> GPUComputePipeline {
    let mut hasher = PipelineHasher::default();
    hasher.write_u32(workgroup_size);

    gpu.device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
        builder = builder.with_config_work_group_size(workgroup_size);

        let input = builder.bind_by(&input_buffer);
        let output = builder.bind_by(&output_buffer);

        let gid = builder.global_invocation_id().x();
        // ... shader logic ...
        output.index(gid).store(result);

        builder  // must return builder
    })
}
```

`PipelineHasher` implements `std::hash::Hasher`. Write all params that affect the generated shader code (workgroup size, feature flags, etc.). The pipeline is cached by hash in `gpu.device`.



## Reference Examples

| Example | File |
|---------|------|
| Compute 101 (prefix sum) | [platform/graphics/webgpu/examples/compute101.rs](platform/graphics/webgpu/examples/compute101.rs) |
| Ray tracing | [shader/ray-tracing/src/test.rs](shader/ray-tracing/src/test.rs) |
| Sampling library (`#[shader_fn]`) | [shader/library/src/sampling.rs](shader/library/src/sampling.rs) |
| Normal mapping | [shader/library/src/normal_mapping.rs](shader/library/src/normal_mapping.rs) |
| Bezier GPU evaluation | [extension/parametric-rendering/src/bezier_surface_device/compute.rs](extension/parametric-rendering/src/bezier_surface_device/compute.rs) |
| Full GPU unit tests | [extension/parametric-rendering/src/bezier_surface_device/tests.rs](extension/parametric-rendering/src/bezier_surface_device/tests.rs) |

## GPU Unit Testing

For buffer creation APIs (`create_gpu_readonly_storage`, `create_gpu_read_write_storage`, `ZeroedArrayByArrayLength`, etc.), see [[shader-edsl-binding-and-typed-container]]. This section covers the compute-specific flow: GPU init → dispatch → readback.

### Test runner

```rust
use rendiation_shader_api::*;
use rendiation_webgpu::*;

#[pollster::test]
async fn my_compute_test() {
    let (gpu, _) = GPU::new(Default::default()).await.unwrap();
    // upload data, build pipeline, dispatch, readback, verify
}
```

`#[pollster::test]` blocks synchronously on the async fn.

### Dispatch and pass-side binding

```rust
let dispatch_x = (total_samples as u32 + workgroup_size - 1) / workgroup_size;

let mut encoder = gpu.create_encoder().with_compute_pass_scoped(|mut pass| {
    BindingBuilder::default()
        .with_bind(&info)       // order must match builder.bind_by() in pipeline
        .with_bind(&cp)
        .with_bind(&binomial)
        .with_bind(&output)
        .setup_compute_pass(&mut pass, &gpu.device, &pipeline);
    pass.dispatch_workgroups(dispatch_x, 1, 1);
});
```

### Readback

```rust
let result = encoder.read_buffer(&gpu.device, &output);
gpu.submit_encoder(encoder);
let result = result.await.unwrap();

let gpu_data: Vec<Vec4<f32>> =
    <[Vec4<f32>]>::from_bytes_into_boxed(&result.read_raw()).into_vec();
```

`read_buffer` schedules GPU→CPU transfer. `submit_encoder` flushes all recorded work. `.await` waits for the transfer.

### Complete example

A minimal compute pipeline that writes `1 + 1` to a buffer and verifies the result:

```rust
use rendiation_algebra::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

fn build_add_pipeline(
    gpu: &GPU,
    output: &StorageBufferDataView<[f32]>,
) -> GPUComputePipeline {
    let mut hasher = PipelineHasher::default();
    gpu.device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
        let output = builder.bind_by(output);
        let gid = builder.global_invocation_id().x();
        let result = val(1.0) + val(1.0);
        output.index(gid).store(result);
        builder
    })
}

#[pollster::test]
async fn one_plus_one() {
    let (gpu, _) = GPU::new(Default::default()).await.unwrap();

    let output = create_gpu_read_write_storage::<[f32]>(
        ZeroedArrayByArrayLength(1), &gpu,
    );

    let pipeline = build_add_pipeline(&gpu, &output);

    let mut encoder = gpu.create_encoder().with_compute_pass_scoped(|mut pass| {
        BindingBuilder::default()
            .with_bind(&output)
            .setup_compute_pass(&mut pass, &gpu.device, &pipeline);
        pass.dispatch_workgroups(1, 1, 1);
    });

    let result = encoder.read_buffer(&gpu.device, &output);
    gpu.submit_encoder(encoder);
    let result = result.await.unwrap();
    let data: Vec<f32> = <[f32]>::from_bytes_into_boxed(&result.read_raw()).into_vec();

    assert!((data[0] - 2.0).abs() < 1e-6, "expected 2.0, got {}", data[0]);
}
```
