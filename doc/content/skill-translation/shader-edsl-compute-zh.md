---
name: shader-edsl-compute
description: >
  rendiation 着色器 EDSL 的计算管线参考。涵盖管线构建、GPU 单元测试、工作组共享/私有内存、屏障、
  内置计算 ID、工作组 uniform 加载、光线追踪(波前计算后端),
  以及工作组归约等计算专用配方。
  构建计算着色器管线或编写 GPU 单元测试时使用。
  阶段无关的语言原语依赖 shader-edsl-core。
  同时依赖 shader-edsl-binding-and-typed-container。
metadata:
  version: "1.1"
  updated: "2026-05-17"
---

rendiation 计算管线参考。核心语言见 `shader-edsl-core`,资源绑定见 `shader-edsl-binding-and-typed-container`,图形管线见 `shader-edsl-graphics`。

```rust
use rendiation_shader_api::*;
```


## 内置计算 ID

| 方法 | 返回类型 |
|--------|-------------|
| `.global_invocation_id()` | `Node<Vec3<u32>>` |
| `.local_invocation_id()` | `Node<Vec3<u32>>` |
| `.local_invocation_index()` | `Node<u32>` |
| `.workgroup_id()` | `Node<Vec3<u32>>` |
| `.workgroup_count()` | `Node<Vec3<u32>>` |
| `.subgroup_invocation_id()` | `Node<u32>`(需要 subgroup 支持) |
| `.subgroup_id()` | `Node<u32>`(需要 subgroup 支持) |
| `.subgroup_size()` | `Node<u32>`(需要 subgroup 支持) |

**工作组大小配置**:`IntoWorkgroupSize` trait,为 `u32`、`(u32, u32)`、`(u32, u32, u32)` 实现


## 屏障

```rust
storage_barrier();     // 存储内存屏障
workgroup_barrier();   // 工作组内存屏障
subgroup_barrier();    // subgroup 屏障(需要 SUBGROUP_BARRIER 特性)
```


## 工作组共享与私有内存

### 工作组共享内存

```rust
// 固定大小
let shared: ShaderPtrOf<Vec4<f32>> = builder.define_workgroup_shared_var::<Vec4<f32>>();

// 主机指定大小的数组(GPU 端视为固定大小,CPU 端为动态)
let shared_arr: ShaderPtrOf<HostDynSizeArray<f32>> =
    builder.define_workgroup_shared_var_host_size_array::<f32>(len);
```

### 工作组 uniform 加载

```rust
// 将工作组内存中的 uniform 值广播到所有 invocation(调用实例)
let uniform_val: Node<f32> = workgroup_uniform_load(ptr);
```

## 常见模式(配方)

### 工作组归约

```rust
let shared: ShaderPtrOf<f32> = builder.define_workgroup_shared_var_host_size_array::<f32>(256);
let lid = builder.local_invocation_id().x();

// 加载到共享内存
shared.index(lid).store(data);
workgroup_barrier();

// 树形归约
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

// 线程 0 持有最终结果
let result = shared.index(val(0_u32)).load();
```

### 按 invocation 计数的循环

```rust
val(256_u32).into_shader_iter().for_each(|i, _| {
    // i: Node<u32>, 0..255
});
```

### 遍历存储缓冲区数组

```rust
let buffer: ShaderPtrOf<[Item]> = builder.bind_by(&resource);
buffer.into_shader_iter().for_each(|item, _| {
    let data = item.load();
    // 处理数据...
});
```

### SSAO 风格:迭代 + 累加

```rust
let result = samples
    .into_shader_iter()
    .clamp_by(sample_count)
    .map(|(_, sample): (_, ShaderReadonlyPtrOf<Vec4<f32>>)| {
        let s = sample.load();
        // 处理采样 ...
        val(0.0)
    })
    .sum();
```

### Subgroup 前缀和

```rust
let inclusive: Node<f32> = value.subgroup_inclusive_add();
let exclusive: Node<f32> = value.subgroup_exclusive_add();
```


## 计算管线模板

```rust
pub fn build_my_pipeline(gpu: &GPU, ...) -> GPUComputePipeline {
    let hasher = shader_hasher_from_marker_ty!(MyPipeline) // 使用唯一的结构体为哈希提供唯一性
    .with_hash((workgroup_size));

    gpu.device.get_or_cache_create_compute_pipeline_by(hasher, |mut builder| {
        builder = builder.with_config_work_group_size(workgroup_size);

        let input = builder.bind_by(&input_buffer);
        let output = builder.bind_by(&output_buffer);

        let gid = builder.global_invocation_id().x();
        // ... 着色器逻辑 ...
        output.index(gid).store(result);

        builder  // 必须返回 builder
    })
}
```

`PipelineHasher` 实现了 `std::hash::Hasher`。将影响生成着色器代码的所有参数(工作组大小、特性开关等)写入其中。管线按哈希缓存在 `gpu.device` 中。



## 参考示例

| 示例 | 文件 |
|---------|------|
| 计算入门(前缀和) | [platform/graphics/webgpu/examples/compute101.rs](../../../../../rendiation/platform/graphics/webgpu/examples/compute101.rs) |
| 光线追踪 | [shader/ray-tracing/src/test.rs](../../../../../rendiation/shader/ray-tracing/src/test.rs) |
| 采样库(`#[shader_fn]`) | [shader/library/src/sampling.rs](../../../../../rendiation/shader/library/src/sampling.rs) |
| 法线映射 | [shader/library/src/normal_mapping.rs](../../../../../rendiation/shader/library/src/normal_mapping.rs) |
| Bezier GPU 求值 | [extension/parametric-rendering/src/bezier_surface_device/compute.rs](../../../../../rendiation/extension/parametric-rendering/src/bezier_surface_device/compute.rs) |
| GPU 单元测试全集 | [extension/parametric-rendering/src/bezier_surface_device/tests.rs](../../../../../rendiation/extension/parametric-rendering/src/bezier_surface_device/tests.rs) |

## GPU 单元测试

缓冲区创建 API(`create_gpu_readonly_storage`、`create_gpu_read_write_storage`、`ZeroedArrayByArrayLength` 等)见 [[shader-edsl-binding-and-typed-container]]。本节介绍计算专用流程:GPU 初始化 → 派发 → 回读。

### 测试运行器

```rust
use rendiation_shader_api::*;
use rendiation_webgpu::*;

#[pollster::test]
async fn my_compute_test() {
    let (gpu, _) = GPU::new(Default::default()).await.unwrap();
    // 上传数据、构建管线、派发、回读、验证
}
```

`#[pollster::test]` 会同步阻塞等待 async 函数完成。

### 派发与通道侧绑定

```rust
let dispatch_x = (total_samples as u32 + workgroup_size - 1) / workgroup_size;

let mut encoder = gpu.create_encoder().with_compute_pass_scoped(|mut pass| {
    BindingBuilder::default()
        .with_bind(&info)       // 顺序必须与管线中的 builder.bind_by() 一致
        .with_bind(&cp)
        .with_bind(&binomial)
        .with_bind(&output)
        .setup_compute_pass(&mut pass, &gpu.device, &pipeline);
    pass.dispatch_workgroups(dispatch_x, 1, 1);
});
```

### 回读

```rust
let result = encoder.read_buffer(&gpu.device, &output);
gpu.submit_encoder(encoder);
let result = result.await.unwrap();

let gpu_data: Vec<Vec4<f32>> =
    <[Vec4<f32>]>::from_bytes_into_boxed(&result.read_raw()).into_vec();
```

`read_buffer` 调度 GPU→CPU 传输。`submit_encoder` 刷新所有已记录的工作。`.await` 等待传输完成。

### 完整示例

一个最小的计算管线,将 `1 + 1` 写入缓冲区并验证结果:

```rust
use rendiation_algebra::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;

fn build_add_pipeline(
    gpu: &GPU,
    output: &StorageBufferDataView<[f32]>,
) -> GPUComputePipeline {
    let hasher = shader_hasher_from_marker_ty!(MyPipeline); // 使用唯一的结构体为哈希提供唯一性
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
