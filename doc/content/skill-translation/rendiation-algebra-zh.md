---
name: rendiation-algebra
description: >
  rendiation 数学库(math/algebra)的参考文档。涵盖 Vec2/Vec3/Vec4、Mat2/Mat3/Mat4、
  Quat、Scalar trait(Float + FloatConst + ScalarConstEval)、通过 scalar_transmute/eval
  构造泛型浮点常量、InnerProductSpace(length2/dot)、VectorSpace,
  以及 SpaceEntity。在编写基于 T: Scalar 的泛型数学代码、使用向量/矩阵,
  或在泛型上下文中创建浮点字面量时使用。
metadata:
  version: "1.0"
  updated: "2026-05-18"
---

`math/algebra` crate 是基础数学库。通过以下方式导入全部内容:

```rust
use rendiation_algebra::*;
```

## Scalar trait

`Scalar` 是一个 trait 别名,它打包了泛型数学代码所需的全部数值能力:

```rust
pub trait Scalar = Float
  + AsPrimitive<i64>
  + FloatConst
  + ScalarConstEval
  + Copy
  + std::fmt::Debug
  + AddAssign<Self> + SubAssign<Self> + DivAssign<Self> + MulAssign<Self>
  + Send + Sync + Default
  + 'static;
```

这意味着 `T: Scalar` 只为 `f32` 和 `f64` 实现。它提供:
- `T::zero()`, `T::one()` — 来自 `num_traits::Zero`/`One`
- `T::abs()`, `T::sqrt()` — 来自 `Float`
- 算术运算符(`+`, `-`, `*`, `/`, `+=`, `-=`, `*=`, `/=`) — 来自 `Float` 与 `*Assign` traits
- 比较(`<`, `>`, `==`) — 来自 `PartialOrd`(经由 `Float`)
- `T::from(small_int)` — 来自 `NumCast`(例如 `T::from(3).expect(...)`)

## 通过 ScalarConstEval 构造泛型浮点常量

`ScalarConstEval` 通过 `f32` 位模式提供编译期的浮点转泛型转换:

```rust
pub trait ScalarConstEval: Sized {
  fn eval<const N: u32>() -> Self;   // f32::from_bits(N).into()
  fn half() -> Self;                 // 0.5
  fn two() -> Self;                  // 2.0
  fn three() -> Self;                // 3.0
}

impl<T: From<f32>> ScalarConstEval for T { ... }
```

辅助函数 `scalar_transmute` 将浮点字面量转换为 `u32` 位模式,用于 const 泛型参数:

```rust
pub const fn scalar_transmute(value: f32) -> u32 { value.to_bits() }
```

**使用模式** — 在泛型代码中写出 `1e-7`:

```rust
let threshold: T = T::eval::<{ scalar_transmute(1e-7) }>();
```

常用常量:`T::half()` (0.5)、`T::two()` (2.0)、`T::one()`、`T::zero()`。

## 向量类型

全部由 `impl_vector!` 宏生成。可用类型:`Vec2<T>`、`Vec3<T>`、`Vec4<T>`。

**构造函数:**
```rust
let v = Vec3::new(x, y, z);
let v = Vec4::new(x, y, z, w);
```

**关键 traits(完整功能均要求 `T: Scalar`):**

| Trait | 提供 | 文件 |
|-------|------|------|
| `InnerProductSpace<T>` | `length2()`, `dot()`, `length()` | [vec/dimension.rs](../../../../../rendiation/math/algebra/src/vec/dimension.rs) |
| `VectorSpace<T>` | `Add`, `Sub`, `Mul<T>`, `Div<T>` | [vec/dimension.rs](../../../../../rendiation/math/algebra/src/vec/dimension.rs) |
| `Functor` | `f_map(f)` | [lib.rs](../../../../../rendiation/math/algebra/src/lib.rs) |

向量算术是逐元素进行的:`a + b`、`a - b`、`a * scalar`、`a / scalar`、`scalar * a`。

## 矩阵类型

`Mat2<T>`、`Mat3<T>`、`Mat4<T>` — 方阵。单位矩阵用 `Mat4::identity()`。

## 四元数

`Quat<T>` — 用于旋转的四元数。

## f32/f64 转换

每个向量/矩阵/四元数类型上都有的方法(由 `f32_f64_convert!` 宏生成,而非 Functor trait 提供):

```rust
v.into_f64()  // Vec3<f32> → Vec3<f64>
v.into_f32()  // Vec3<f64> → Vec3<f32>
```

## SpaceEntity

用于可以被矩阵变换的类型:

```rust
pub trait SpaceEntity<T: Scalar, const D: usize> {
  type Matrix: SquareMatrixDimension<D>;
  fn apply_matrix(&mut self, mat: Self::Matrix) -> &mut Self;
}
```

## 关键文件

| 文件 | 内容 |
|------|------|
| [math/algebra/src/lib.rs](../../../../../rendiation/math/algebra/src/lib.rs) | `Scalar` trait 别名、`ScalarConstEval`、`scalar_transmute`、`f32_f64_convert!` |
| [math/algebra/src/vec/mod.rs](../../../../../rendiation/math/algebra/src/vec/mod.rs) | `impl_vector!` 宏、`vec2`/`vec3`/`vec4` 构造函数 |
| [math/algebra/src/vec/dimension.rs](../../../../../rendiation/math/algebra/src/vec/dimension.rs) | `InnerProductSpace`、`VectorSpace`、`Vector` traits |
| [math/algebra/src/vec/vec3.rs](../../../../../rendiation/math/algebra/src/vec/vec3.rs) | `Vec3` 结构体、`dot_impl` |
| [math/algebra/src/vec/vec4.rs](../../../../../rendiation/math/algebra/src/vec/vec4.rs) | `Vec4` 结构体 |
| [math/algebra/src/mat/](../../../../../rendiation/math/algebra/src/mat/) | `Mat2`、`Mat3`、`Mat4` |
| [math/algebra/src/quat.rs](../../../../../rendiation/math/algebra/src/quat.rs) | `Quat` |
