---
name: shader-edsl-core
description: >
  rendiation 着色器 EDSL 的核心语言参考——与阶段无关的基础构建块。
  涵盖 Node<T>、值构造、着色器结构体、内存布局、控制流、GPU 端
  迭代、纹理操作、原子操作、Subgroup 操作、#[shader_fn]、数学函数
  以及向量/矩阵运算。在编写任何着色器表达式、结构体或逻辑时使用——
  无论管线阶段为何。
metadata:
  version: "1.0"
  updated: "2026-05-16"
---

Rendiation 使用基于 Rust 的 EDSL(嵌入式领域特定语言,embedded domain-specific language)通过 `naga` 后端生成 WGSL 风格的着色器。本文档是**与阶段无关的核心语言**参考——涵盖类型、表达式、控制流与内置函数。管线集成(顶点/片元/计算阶段、语义、绑定)参见 `shader-edsl-graphics` 与 `shader-edsl-compute`。

```rust
use rendiation_shader_api::*;
```


## 核心概念

### Node<T> — 类型化着色器句柄

`Node<T>` 是所有着色器值的统一句柄,实现 `Copy` + `Clone`。数学运算通过 Rust 的 trait 系统完成。

```rust
// 创建常量
let x: Node<f32> = val(1.0);
let v: Node<Vec3<f32>> = val(Vec3::one());

// 算术运算(经由 std::ops 运算符重载)
let sum = x + val(2.0);
let scaled = v * x;
let cmp: Node<bool> = x.less_than(val(3.0));

// 零初始化值
let zero: Node<Vec3<f32>> = zeroed_val();

// 可变局部变量
let slot: ShaderPtrOf<Vec3<f32>> = make_local_var::<Vec3<f32>>();
slot.store(val(Vec3::new(1.0, 0.0, 0.0)));
let loaded: Node<Vec3<f32>> = slot.load();

// 或从 Node 值初始化局部变量
let slot = val(Vec3::new(1.0, 0.0, 0.0)).make_local_var();

// 定长局部数组
let arr: ShaderPtrOf<[f32; 16]> = make_local_var::<[f32; 16]>();
```

### 数组类型与索引

| 数组类型 | 索引类型 | `.index(idx)` 返回 | 示例 |
|------------|-----------|----------------------|---------|
| `ShaderPtrOf<[T; N]>` | `Node<u32>` | `ShaderPtrOf<T>` | `arr.index(i).store(v)` / `.load()` |
| `ShaderReadonlyPtrOf<[T; N]>` | `Node<u32>` | `ShaderReadonlyPtrOf<T>` | `arr.index(i).load()` |
| `ShaderPtrOf<[T]>`(动态) | `Node<u32>` | `ShaderPtrOf<T>` | 存储缓冲区读写 |
| `ShaderReadonlyPtrOf<[T]>`(动态) | `Node<u32>` | `ShaderReadonlyPtrOf<T>` | 存储缓冲区只读 |

### 关键类型参考

| 类型 | 含义 |
|------|---------|
| `Node<T>` | 不可变着色器值句柄(Copy) |
| `ShaderPtrOf<T>` | 可变指针(支持 store) |
| `ShaderReadonlyPtrOf<T>` | 只读指针(仅 load) |
| `ENode<T>` | 展开后的结构体字段(`<T as ShaderStructuralNodeType>::Instance`) |
| `BindingNode<T>` | 绑定资源句柄(`Node<ShaderBinding<T>>`) |

### ENode:结构体字段级访问

```rust
// 从缓冲区加载并展开
let raw: Node<MyUniform> = buffer.load();
let fields: ENode<MyUniform> = raw.expand();

// 修改字段并重建
let modified = ENode::<MyUniform> {
    roughness: fields.roughness * val(0.5),
    ..fields  // Rust 结构体更新语法
}.construct();
```


## 着色器结构体与 ENode

### 定义着色器结构体

```rust
#[repr(C)]
#[derive(Clone, Copy, Debug, ShaderStruct)]
struct MyMaterial {
    pub base_color: Vec3<f32>,
    pub roughness: f32,
    pub metallic: f32,
}
```

`#[derive(ShaderStruct)]` 自动生成:

```rust
struct MyMaterialShaderInstance {
    pub base_color: Node<Vec3<f32>>,
    pub roughness: Node<f32>,
    pub metallic: Node<f32>,
}
```

- `ENode<MyMaterial>` — `MyMaterialShaderInstance` 的别名,即所有字段均为句柄的结构体
- `MyMaterialShaderAPIInstance` — 字段访问器(`MyMaterial::base_color(node)`)
- `MyMaterialShaderAPIPtrInstance` / `MyMaterialShaderAPIReadonlyPtrInstance` — 指针视图

### ENode 展开与构造

```rust
// 统一缓冲区绑定 → 加载 → 展开
let mat: Node<MyMaterial> = binding.bind_by(&self.material).load();
let f = mat.expand();

// 使用字段
let color = f.base_color;
let rough = f.roughness;

// 修改字段并重建
let new_mat = ENode::<MyMaterial> {
    roughness: rough * val(0.5),
    ..f
}.construct();
```

### 直接访问结构体字段(不展开)

```rust
// 从 Node<MyMaterial> 访问字段(经由生成的 ShaderAPIInstance)
let color: Node<Vec3<f32>> = MyMaterial::base_color(mat);
```


## 内存布局标注

### std140(统一缓冲区)与 std430(存储缓冲区)

```rust
#[repr(C)]
#[std140_layout]     // 统一缓冲区必须标注
#[derive(Clone, Copy, ShaderStruct)]
struct MyUniform {
    pub color: Vec3<f32>,
    pub scale: f32,
}

#[repr(C)]
#[std430_layout]     // 存储缓冲区必须标注
#[derive(Clone, Copy, ShaderStruct)]
struct MyStorage {
    pub data: Vec4<f32>,
}
```

| 标注 | 对齐方式 | 用途 |
|------------|-----------|----------|
| `#[std140_layout]` | 16 字节 | 统一缓冲区 |
| `#[std430_layout]` | 自然对齐 | 存储缓冲区 |
| 无 | — | 着色器内部使用(非缓冲区) |

### std140 特例

- **Bool**:不能在 std140 与 std430 中直接用作字段;请改用 `Bool`。
- std140 兼容的 mat3 请使用 `Shader16PaddedMat3` 代替 `Mat3<f32>`
- std140 兼容的定长数组请使用 `Shader140Array<T, N>` 代替 `[T, N]`


## 控制流

### if_by / else_if / else_by

```rust
if_by(a.less_than(val(0.0)), || {
    // then 分支
})
.else_if(a.greater_than(val(1.0)), || {
    // else if 分支
})
.else_by(|| {
    // else 分支
});
// 注意:`.else_if()` 与 `.else_by()` 可以省略
// 注意:只要使用了 `.else_if()`,结尾就必须调用 `.else_by()` 或 `.else_over()`。

```

### 三元表达式(基于分支的选择)

```rust
// 表达式上下文中使用 select_branched 优于 if_by
let result: Node<Vec3<f32>> = condition.select_branched(
    || val(Vec3::new(1.0, 0.0, 0.0)),   // 为真
    || val(Vec3::new(0.0, 0.0, 1.0)),   // 为假
);
```

### loop_by

```rust
loop_by(|cx| {
    // 循环体
    if_by(should_stop, || {
        cx.do_break();
    });
    // 或者跳过本次迭代:cx.do_continue();
});
```

### switch_by

```rust
switch_by(selector)   // selector:Node<u32> 或 Node<i32>
    .case(0, || { /* ... */ })
    .case(1, || { /* ... */ })
    .end_with_default(|| { /* 默认分支 */ });
    // 必须调用 .end_with_default()!
```

### return

```rust
return_value(Some(value));  // 返回一个值
do_return();               // 返回 void
```

很少使用,只能在函数上下文中使用


## GPU 端迭代(`into_shader_iter`)

将统一/存储缓冲区数组转换为 GPU 端可迭代对象。

### 基本用法

```rust
// 计数循环
val(10_u32).into_shader_iter().for_each(|i, _| {
    // i: Node<u32>,从 0 到 9
});

// 遍历存储缓冲区数组
items.into_shader_iter().for_each(|item, _| {
    let data = item.load();
    // ...
});
```

### 链式操作

```rust
samples
    .into_shader_iter()
    .clamp_by(sample_count.x())   // 动态限制迭代次数
    .map(|(i, sample): (_, ShaderReadonlyPtrOf<Vec4<f32>>)| {
        // i: Node<u32>,sample:指针
        sample.load()
    })
    .sum()  // 累加
```

### 支持的适配器

| 方法 | 用途 |
|--------|---------|
| `.map(f)` | 映射 |
| `.filter(pred)` | 过滤 |
| `.filter_map(f)` | 过滤 + 映射 |
| `.zip(other)` | 压缩两个迭代器 |
| `.enumerate()` | 附带索引 |
| `.take_while(pred)` | 条件截断 |
| `.clamp_by(count)` | 限制迭代次数 |
| `.flat_map(f)` | 扁平映射 |
| `.for_each(f)` | 迭代 |
| `.sum()` | 求和 |

### 迭代来源

| 类型 | `into_shader_iter()` 来源 |
|------|---------------------------|
| `u32` / `Node<u32>` | 0..n 计数循环 |
| `Node<Vec2<u32>>` | `ForRange`:from..to |
| StaticLengthArrayView | 编译期已知长度的数组 |
| DynLengthArrayView | 运行时长度数组 |


## 纹理操作

### 采样纹理

```rust
// 基础采样(隐式 LOD)
let color: Node<Vec4<f32>> = texture.sample(sampler, uv);

// 零级采样(无 mipmap 或显式 level 0)
let color = texture.sample_zero_level(sampler, uv);

// 显式 LOD
let color = texture
    .build_sample_call(sampler, uv)
    .with_level(level)
    .sample();

// 带 LOD 偏移
let color = texture
    .build_sample_call(sampler, uv)
    .with_level_bias(bias)
    .sample();

// 带梯度
let color = texture
    .build_sample_call(sampler, uv)
    .with_level_grad(ddx, ddy)
    .sample();

// Gather(取四个纹素)
let gathered = texture
    .build_sample_call(sampler, uv)
    .gather(GatherChannel::Red);
```

### 直接加载(无采样器的纹素访问)

```rust
// 2D 纹理
let value = texture.load_texel(coord);

// 2D 数组
let value = texture.load_texel_layer(coord, layer);

// 多重采样
let value = texture.load_texel_multi_sample_index(coord, sample_index);
```

### 存储纹理(可读写)

```rust
// 读取
let value = storage_tex.load_texel(coord);

// 写入
storage_tex.write_texel(coord, value);
storage_tex.write_texel_index(coord, index, value); // 数组层
```

### 纹理类型别名

| 别名 | 完整类型 |
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

### 纹理元数据查询

```rust
let layers: Node<u32> = texture.texture_number_layers();
let levels: Node<u32> = texture.texture_number_levels();
let dims: Node<Vec2<u32>> = texture.texture_dimension_2d(None);  // None 表示基础级别
```


## 原子操作

```rust
// 原子类型的指针视图
let atomic_ptr: ShaderPtrOf<DeviceAtomic<T>> = /* 来自缓冲区或共享内存 */;

// 基础原子操作
let old: Node<u32> = atomic_ptr.atomic_load();
atomic_ptr.atomic_store(val(42));
let old: Node<u32> = atomic_ptr.atomic_exchange(val(0));

// 算术原子操作
let old = atomic_ptr.atomic_add(val(1));
let old = atomic_ptr.atomic_sub(val(1));
let old = atomic_ptr.atomic_min(val(10));
let old = atomic_ptr.atomic_max(val(100));

// 位运算原子操作
let old = atomic_ptr.atomic_and(val(0xFF));
let old = atomic_ptr.atomic_or(val(0x01));
let old = atomic_ptr.atomic_xor(val(0xFF));

```


## Subgroup 操作

### 集体归约(Collective Reduce)

```rust
let sum: Node<f32> = value.subgroup_add();
let product: Node<f32> = value.subgroup_mul();
let min: Node<f32> = value.subgroup_min();
let max: Node<f32> = value.subgroup_max();
```

### 扫描(Scan)

```rust
let inclusive: Node<f32> = value.subgroup_inclusive_add();
let exclusive: Node<f32> = value.subgroup_exclusive_add();
let excl_mul: Node<f32> = value.subgroup_exclusive_mul();
let incl_mul: Node<f32> = value.subgroup_inclusive_mul();
```

### 通信

```rust
let val: Node<f32> = value.subgroup_broadcast(id);       // 广播到所有线程
let shuffled: Node<f32> = value.subgroup_shuffle(id);     // 洗牌
let up: Node<f32> = value.subgroup_shuffle_up(delta);     // 向上洗牌
let down: Node<f32> = value.subgroup_shuffle_down(delta); // 向下洗牌
```

### 布尔

```rust
let all: Node<bool> = condition.subgroup_all();            // 全部为真
let any: Node<bool> = condition.subgroup_any();            // 任一为真
let ballot: Node<Vec4<u32>> = condition.subgroup_ballot(); // 位掩码
```

### 整数位运算

```rust
let and: Node<u32> = value.subgroup_and();
let or: Node<u32> = value.subgroup_or();
let xor: Node<u32> = value.subgroup_xor();
```


## `#[shader_fn]` 可复用函数

定义可在 GPU 上调用且自动去重的可复用函数。

```rust
#[shader_fn]
fn my_mix(a: Node<Vec3<f32>>, b: Node<Vec3<f32>>, t: Node<f32>) -> Node<Vec3<f32>> {
    a * (val(1.0) - t) + b * t
}

// GPU 端调用:
let result = my_mix_fn(color1, color2, factor);
```

**规则**:

- 参数必须是 `Node<T>` 类型
- 返回类型自动推断(标注可选)
- 可以调用其他 `#[shader_fn]` 函数
- 内部可以使用控制流(`if_by`、`loop_by` 等)
- 以 `_fn` 后缀调用(由宏生成)


## 内置数学函数

所有方法都直接在 `Node<T>` 上调用。

### 算术 / 比较

| 方法 | 描述 |
|--------|-------------|
| `.abs()` | 绝对值 |
| `.min(v)` | 最小值 |
| `.max(v)` | 最大值 |
| `.clamp(low, high)` | 钳制 |
| `.saturate()` | 钳制到 [0, 1] |
| `.sign()` | 符号 |
| `.step(edge)` | 阶梯 |
| `.smoothstep(low, high)` | 平滑阶梯 |
| `.mix(a, b, t)` | 混合(a、b 为同类型 Node,t 为系数) |
| `.equals(v)` | 等于 |
| `.less_than(v)` | 小于 |
| `.greater_than(v)` | 大于 |
| `.not_equals(v)` | 不等于 |

### 向量

| 方法 | 描述 |
|--------|-------------|
| `.dot(v)` | 点积 |
| `.cross(v)` | 叉积(仅 Vec3) |
| `.normalize()` | 归一化 |
| `.length()` | 长度 |
| `.distance(v)` | 距离 |
| `.reflect(n)` | 反射 |
| `.refract(n, eta)` | 折射 |

### 矩阵

| 方法 | 描述 |
|--------|-------------|
| `.transpose()` | 矩阵转置 |

### 数学函数

`.sin()`, `.cos()`, `.tan()`, `.asin()`, `.acos()`, `.atan()`, `.atan2(other)`,
`.sinh()`, `.cosh()`, `.tanh()`,
`.exp()`, `.exp2()`, `.ln()` (log_e), `.log2()`,
`.pow(exp)`, `.sqrt()`, `.inverse_sqrt()`,
`.floor()`, `.ceil()`, `.round()`, `.fract()`, `.trunc()`

### 布尔 / 选择

| 表达式 | 描述 |
|------------|-------------|
| `x.select(true_val, false_val)` | 条件选择 |
| `x.all()` | `Node<Vec<bool>> -> Node<bool>` — 全部为真 |
| `x.any()` | `Node<Vec<bool>> -> Node<bool>` — 任一为真 |
| `x.and(y)` | 逻辑与 |
| `x.or(y)` | 逻辑或 |
| `x.not()` | 逻辑非 |

### 屏幕空间导数

```rust
let dx: Node<Vec3<f32>> = value.dpdx();   // dFdx
let dy: Node<Vec3<f32>> = value.dpdy();   // dFdy
let w: Node<Vec3<f32>> = value.fwidth();  // fwidth
```

### 类型转换

```rust
let f: Node<f32> = int_val.into_f32();
let u: Node<u32> = float_val.into_u32();
let i: Node<i32> = float_val.into_i32();
let bits: Node<u32> = float_val.bitcast::<u32>();
```

### 向量布尔运算

```rust
// 逐分量选择
let result = mask.select(if_true, if_false);
// mask: Node<VecN<bool>>,if_true/if_false: VecN<T>
```


## 向量与矩阵构造

### 向量构造

```rust
// 从标量构造
let v3: Node<Vec3<f32>> = val(Vec3::new(1.0, 2.0, 3.0));

// 从分量构造
let v: Node<Vec4<f32>> = (val(1.0), val(2.0), val(3.0), val(1.0)).into();
```

### 分量重排(Swizzle)

```rust
// 向量分量重排(x/y/z/w 分量)
let xy: Node<Vec2<f32>> = vec3.xy();
let xyz: Node<Vec3<f32>> = vec4.xyz();
let yx: Node<Vec2<f32>> = vec2.yx();
let zyx: Node<Vec3<f32>> = vec3.zyx();
let x: Node<f32> = vec4.x();

// 颜色通道
let rgb: Node<Vec3<f32>> = vec4.rgb();
let a: Node<f32> = vec4.a();

// Splat(广播)
let v4 = val(1.0).splat::<Vec4<f32>>();  // (1, 1, 1, 1)
```

### 矩阵构造

```rust
// 从 3 个列向量构造
let m: Node<Mat3<f32>> = (col0, col1, col2).into();

// 从 4 个列向量构造
let m: Node<Mat4<f32>> = (col0, col1, col2, col3).into();

// 矩阵访问
let col: Node<Vec4<f32>> = mat.x();
let pos: Node<Vec3<f32>> = mat.position();   // mat4 最后一列(位置)
let fwd: Node<Vec3<f32>> = mat.forward();    // mat4 第三列(z)
let rot: Node<Mat3<f32>> = mat.shrink_to_3(); // mat4 → mat3
```


## 注意事项

- 没有枚举/和类型(enum / sum type),请使用 `Node<bool>` 标志配合 `.select()` / `.select_branched()`,或使用 `switch_by`
- API 依赖线程局部状态,**不要**跨线程调用
