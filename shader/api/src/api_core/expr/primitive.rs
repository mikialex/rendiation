use crate::*;

#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub enum ScalarType {
  F32,
  U32,
  I32,
  Bool,
}

impl ScalarType {
  pub fn byte_count(self) -> u32 {
    match self {
      ScalarType::Bool => 1,
      ScalarType::F32 | ScalarType::U32 | ScalarType::I32 => 4,
    }
  }
}

#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub enum VectorSize {
  /// 2D vector
  Bi = 2,
  /// 3D vector
  Tri = 3,
  /// 4D vector
  Quad = 4,
}

#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub enum PrimitiveShaderValueType {
  Scalar(ScalarType),
  Vector {
    size: VectorSize,
    scalar: ScalarType,
  },
  Matrix {
    columns: VectorSize,
    rows: VectorSize,
    scalar: ScalarType,
  },
}

impl PrimitiveShaderValueType {
  pub fn u32() -> Self {
    Self::Scalar(ScalarType::U32)
  }
  pub fn i32() -> Self {
    Self::Scalar(ScalarType::I32)
  }
  pub fn f32() -> Self {
    Self::Scalar(ScalarType::F32)
  }
  pub fn bool() -> Self {
    Self::Scalar(ScalarType::Bool)
  }
  pub fn vec2<T: ShaderScalarType>() -> Self {
    Self::vector(VectorSize::Bi, T::scalar_type())
  }
  pub fn vec3<T: ShaderScalarType>() -> Self {
    Self::vector(VectorSize::Tri, T::scalar_type())
  }
  pub fn vec4<T: ShaderScalarType>() -> Self {
    Self::vector(VectorSize::Quad, T::scalar_type())
  }
  pub fn mat2<T: ShaderScalarType>() -> Self {
    Self::square_matrix(VectorSize::Bi, T::scalar_type())
  }
  pub fn mat3<T: ShaderScalarType>() -> Self {
    Self::square_matrix(VectorSize::Tri, T::scalar_type())
  }
  pub fn mat4<T: ShaderScalarType>() -> Self {
    Self::square_matrix(VectorSize::Quad, T::scalar_type())
  }

  pub const fn vector(size: VectorSize, scalar: ScalarType) -> Self {
    Self::Vector { size, scalar }
  }

  pub const fn square_matrix(size: VectorSize, scalar: ScalarType) -> Self {
    Self::Matrix {
      columns: size,
      rows: size,
      scalar,
    }
  }

  pub fn scalar(self) -> ScalarType {
    match self {
      PrimitiveShaderValueType::Scalar(scalar) => scalar,
      PrimitiveShaderValueType::Vector { scalar, .. } => scalar,
      PrimitiveShaderValueType::Matrix { scalar, .. } => scalar,
    }
  }

  pub fn vertex_out_could_interpolated(self) -> bool {
    match self {
      PrimitiveShaderValueType::Scalar(scalar)
      | PrimitiveShaderValueType::Vector { scalar, .. } => scalar == ScalarType::F32,
      PrimitiveShaderValueType::Matrix { .. } => false,
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarValue {
  F32(f32),
  U32(u32),
  I32(i32),
  Bool(bool),
}

impl ScalarValue {
  pub fn ty(self) -> ScalarType {
    match self {
      ScalarValue::F32(_) => ScalarType::F32,
      ScalarValue::U32(_) => ScalarType::U32,
      ScalarValue::I32(_) => ScalarType::I32,
      ScalarValue::Bool(_) => ScalarType::Bool,
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarValueArray<T> {
  Bi([T; 2]),
  Tri([T; 3]),
  Quad([T; 4]),
}

impl<T> ScalarValueArray<T> {
  #[allow(clippy::len_without_is_empty)]
  pub fn len(&self) -> usize {
    match self {
      ScalarValueArray::Bi(_) => 2,
      ScalarValueArray::Tri(_) => 3,
      ScalarValueArray::Quad(_) => 4,
    }
  }

  pub fn as_slice(&self) -> &[T] {
    match self {
      ScalarValueArray::Bi(v) => v,
      ScalarValueArray::Tri(v) => v,
      ScalarValueArray::Quad(v) => v,
    }
  }

  pub fn iter(&self) -> impl Iterator<Item = &T> {
    self.as_slice().iter()
  }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PrimitiveShaderValue {
  Scalar(ScalarValue),
  Vector {
    size: VectorSize,
    scalar: ScalarType,
    data: ScalarValueArray<ScalarValue>,
  },
  Matrix {
    columns: VectorSize,
    rows: VectorSize,
    scalar: ScalarType,
    data: ScalarValueArray<ScalarValueArray<ScalarValue>>,
  },
}

impl PrimitiveShaderValue {
  pub fn ty(&self) -> PrimitiveShaderValueType {
    match self {
      PrimitiveShaderValue::Scalar(v) => PrimitiveShaderValueType::Scalar(v.ty()),
      PrimitiveShaderValue::Vector { size, scalar, .. } => {
        PrimitiveShaderValueType::vector(*size, *scalar)
      }
      PrimitiveShaderValue::Matrix {
        columns,
        rows,
        scalar,
        ..
      } => PrimitiveShaderValueType::Matrix {
        columns: *columns,
        rows: *rows,
        scalar: *scalar,
      },
    }
  }

  pub fn into_raw_node(self) -> ShaderNodeRawHandle {
    fn scalar_raw_node(v: ScalarValue) -> ShaderNodeRawHandle {
      match v {
        ScalarValue::Bool(v) => val(v).handle(),
        ScalarValue::U32(v) => val(v).handle(),
        ScalarValue::I32(v) => val(v).handle(),
        ScalarValue::F32(v) => val(v).handle(),
      }
    }

    match self {
      PrimitiveShaderValue::Scalar(v) => scalar_raw_node(v),
      PrimitiveShaderValue::Vector { size, scalar, data } => {
        let target =
          ShaderSizedValueType::Primitive(PrimitiveShaderValueType::vector(size, scalar));
        ShaderNodeExpr::Compose {
          target,
          parameters: data.iter().map(|v| scalar_raw_node(*v)).collect(),
        }
        .insert_api_raw()
      }
      PrimitiveShaderValue::Matrix {
        columns,
        rows,
        scalar,
        data,
      } => {
        let target = ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Matrix {
          columns,
          rows,
          scalar,
        });
        ShaderNodeExpr::Compose {
          target,
          parameters: data
            .iter()
            .flat_map(|column| column.iter())
            .map(|v| scalar_raw_node(*v))
            .collect(),
        }
        .insert_api_raw()
      }
    }
  }
}

impl From<PrimitiveShaderValue> for PrimitiveShaderValueType {
  fn from(v: PrimitiveShaderValue) -> Self {
    v.ty()
  }
}

pub trait ShaderScalarType: PrimitiveShaderNodeType + Copy + Into<ScalarValue> {
  fn scalar_type() -> ScalarType;
}
impl ShaderScalarType for bool {
  fn scalar_type() -> ScalarType {
    ScalarType::Bool
  }
}
impl ShaderScalarType for u32 {
  fn scalar_type() -> ScalarType {
    ScalarType::U32
  }
}
impl ShaderScalarType for i32 {
  fn scalar_type() -> ScalarType {
    ScalarType::I32
  }
}
impl ShaderScalarType for f32 {
  fn scalar_type() -> ScalarType {
    ScalarType::F32
  }
}

/// marker trait for numeric scalar types, excluding bool
pub trait ShaderNumericScalarType: ShaderScalarType {}
impl ShaderNumericScalarType for u32 {}
impl ShaderNumericScalarType for i32 {}
impl ShaderNumericScalarType for f32 {}

impl From<bool> for ScalarValue {
  fn from(v: bool) -> Self {
    ScalarValue::Bool(v)
  }
}
impl From<u32> for ScalarValue {
  fn from(v: u32) -> Self {
    ScalarValue::U32(v)
  }
}
impl From<i32> for ScalarValue {
  fn from(v: i32) -> Self {
    ScalarValue::I32(v)
  }
}
impl From<f32> for ScalarValue {
  fn from(v: f32) -> Self {
    ScalarValue::F32(v)
  }
}

impl From<bool> for PrimitiveShaderValue {
  fn from(v: bool) -> Self {
    PrimitiveShaderValue::Scalar(v.into())
  }
}
impl From<u32> for PrimitiveShaderValue {
  fn from(v: u32) -> Self {
    PrimitiveShaderValue::Scalar(v.into())
  }
}
impl From<i32> for PrimitiveShaderValue {
  fn from(v: i32) -> Self {
    PrimitiveShaderValue::Scalar(v.into())
  }
}
impl From<f32> for PrimitiveShaderValue {
  fn from(v: f32) -> Self {
    PrimitiveShaderValue::Scalar(v.into())
  }
}

macro_rules! primitive_value_from_vector {
  ($ty: ty, $size: ident, $array_len: tt) => {
    impl<T: ShaderScalarType + Copy + Into<ScalarValue>> From<$ty> for PrimitiveShaderValue {
      fn from(v: $ty) -> Self {
        let arr: [T; $array_len] = v.into();
        let data = ScalarValueArray::$size(arr.map(Into::into));
        PrimitiveShaderValue::Vector {
          size: VectorSize::$size,
          scalar: T::scalar_type(),
          data,
        }
      }
    }
  };
}

macro_rules! primitive_value_from_matrix {
  ($ty: ty, $columns: ident, $column_len: tt, $rows: ident, $row_len: tt) => {
    impl<T: ShaderScalarType + Copy + Into<ScalarValue>> From<$ty> for PrimitiveShaderValue {
      fn from(v: $ty) -> Self {
        let arr: [ScalarValue; $column_len * $row_len] = {
          let arr: [T; $column_len * $row_len] = v.into();
          arr.map(Into::into)
        };
        let columns: [[ScalarValue; $row_len]; $column_len] = arr
          .chunks_exact($row_len)
          .map(|c| <[ScalarValue; $row_len]>::try_from(c).unwrap())
          .collect::<Vec<_>>()
          .try_into()
          .unwrap();
        let data = ScalarValueArray::$columns(columns.map(ScalarValueArray::$rows));
        PrimitiveShaderValue::Matrix {
          columns: VectorSize::$columns,
          rows: VectorSize::$rows,
          scalar: T::scalar_type(),
          data,
        }
      }
    }
  };
}

primitive_value_from_vector!(Vec2<T>, Bi, 2);
primitive_value_from_vector!(Vec3<T>, Tri, 3);
primitive_value_from_vector!(Vec4<T>, Quad, 4);
primitive_value_from_matrix!(Mat2<T>, Bi, 2, Bi, 2);
primitive_value_from_matrix!(Mat3<T>, Tri, 3, Tri, 3);
primitive_value_from_matrix!(Mat4<T>, Quad, 4, Quad, 4);
primitive_value_from_matrix!(Mat4x3<T>, Quad, 4, Tri, 3);
primitive_value_from_matrix!(Mat2x3<T>, Bi, 2, Tri, 3);
primitive_value_from_matrix!(Mat2x4<T>, Bi, 2, Quad, 4);
primitive_value_from_matrix!(Mat3x2<T>, Tri, 3, Bi, 2);
primitive_value_from_matrix!(Mat3x4<T>, Tri, 3, Quad, 4);
primitive_value_from_matrix!(Mat4x2<T>, Quad, 4, Bi, 2);

// scalars are concrete types so they can not be grouped into one generic impl,
// vec and mat use generic impl over T: ShaderScalarType to cover all supported scalar types.
macro_rules! impl_scalar_primitive_node_type {
  ($ty: ty, $scalar: ident) => {
    impl ShaderNodeSingleType for $ty {
      fn single_ty() -> ShaderValueSingleType {
        ShaderValueSingleType::Sized(ShaderSizedValueType::Primitive(
          PrimitiveShaderValueType::Scalar(ScalarType::$scalar),
        ))
      }
    }
    impl ShaderNodeType for $ty {
      fn ty() -> ShaderValueType {
        ShaderValueType::Single(Self::single_ty())
      }
    }
    impl ShaderSizedValueNodeType for $ty {
      fn sized_ty() -> ShaderSizedValueType {
        ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Scalar(ScalarType::$scalar))
      }
      fn to_value(&self) -> ShaderStructFieldInitValue {
        ShaderStructFieldInitValue::Primitive(self.to_primitive())
      }
    }
    impl PrimitiveShaderNodeType for $ty {
      fn primitive_ty() -> PrimitiveShaderValueType {
        PrimitiveShaderValueType::Scalar(ScalarType::$scalar)
      }
      fn to_primitive(&self) -> PrimitiveShaderValue {
        PrimitiveShaderValue::from(*self)
      }
    }
  };
}

macro_rules! impl_vector_primitive_node_type {
  ($ty: ident, $size: ident) => {
    impl<T> ShaderNodeSingleType for $ty<T>
    where
      T: ShaderScalarType,
    {
      fn single_ty() -> ShaderValueSingleType {
        ShaderValueSingleType::Sized(ShaderSizedValueType::Primitive(
          PrimitiveShaderValueType::vector(VectorSize::$size, T::scalar_type()),
        ))
      }
    }
    impl<T> ShaderNodeType for $ty<T>
    where
      T: ShaderScalarType,
    {
      fn ty() -> ShaderValueType {
        ShaderValueType::Single(Self::single_ty())
      }
    }
    impl<T> ShaderSizedValueNodeType for $ty<T>
    where
      T: ShaderScalarType + Into<ScalarValue>,
    {
      fn sized_ty() -> ShaderSizedValueType {
        ShaderSizedValueType::Primitive(PrimitiveShaderValueType::vector(
          VectorSize::$size,
          T::scalar_type(),
        ))
      }
      fn to_value(&self) -> ShaderStructFieldInitValue {
        ShaderStructFieldInitValue::Primitive(self.to_primitive())
      }
    }
    impl<T> PrimitiveShaderNodeType for $ty<T>
    where
      T: ShaderScalarType + Into<ScalarValue>,
    {
      fn primitive_ty() -> PrimitiveShaderValueType {
        PrimitiveShaderValueType::vector(VectorSize::$size, T::scalar_type())
      }
      fn to_primitive(&self) -> PrimitiveShaderValue {
        PrimitiveShaderValue::from(*self)
      }
    }
  };
}

/// The WGSL matrix types (matCxR, C columns and R rows), `Transposed` is the matRxC type.
pub trait ShaderMatrixType: PrimitiveShaderNodeType {
  type Transposed: PrimitiveShaderNodeType;
}

// matrix element type can only be float in WGSL
macro_rules! impl_matrix_primitive_node_type {
  ($ty: ident, $columns: ident, $rows: ident, $transposed: ident) => {
    impl<T> ShaderMatrixType for $ty<T>
    where
      T: ShaderFloatType + Into<ScalarValue>,
    {
      type Transposed = $transposed<T>;
    }
    impl<T> ShaderNodeSingleType for $ty<T>
    where
      T: ShaderFloatType + Into<ScalarValue>,
    {
      fn single_ty() -> ShaderValueSingleType {
        ShaderValueSingleType::Sized(ShaderSizedValueType::Primitive(
          PrimitiveShaderValueType::Matrix {
            columns: VectorSize::$columns,
            rows: VectorSize::$rows,
            scalar: T::scalar_type(),
          },
        ))
      }
    }
    impl<T> ShaderNodeType for $ty<T>
    where
      T: ShaderFloatType + Into<ScalarValue>,
    {
      fn ty() -> ShaderValueType {
        ShaderValueType::Single(Self::single_ty())
      }
    }
    impl<T> ShaderSizedValueNodeType for $ty<T>
    where
      T: ShaderFloatType + Into<ScalarValue>,
    {
      fn sized_ty() -> ShaderSizedValueType {
        ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Matrix {
          columns: VectorSize::$columns,
          rows: VectorSize::$rows,
          scalar: T::scalar_type(),
        })
      }
      fn to_value(&self) -> ShaderStructFieldInitValue {
        ShaderStructFieldInitValue::Primitive(self.to_primitive())
      }
    }
    impl<T> PrimitiveShaderNodeType for $ty<T>
    where
      T: ShaderFloatType + Into<ScalarValue>,
    {
      fn primitive_ty() -> PrimitiveShaderValueType {
        PrimitiveShaderValueType::Matrix {
          columns: VectorSize::$columns,
          rows: VectorSize::$rows,
          scalar: T::scalar_type(),
        }
      }
      fn to_primitive(&self) -> PrimitiveShaderValue {
        PrimitiveShaderValue::from(*self)
      }
    }
  };
}

impl_scalar_primitive_node_type!(bool, Bool);
impl_scalar_primitive_node_type!(u32, U32);
impl_scalar_primitive_node_type!(i32, I32);
impl_scalar_primitive_node_type!(f32, F32);
impl_vector_primitive_node_type!(Vec2, Bi);
impl_vector_primitive_node_type!(Vec3, Tri);
impl_vector_primitive_node_type!(Vec4, Quad);
impl_matrix_primitive_node_type!(Mat2, Bi, Bi, Mat2);
impl_matrix_primitive_node_type!(Mat3, Tri, Tri, Mat3);
impl_matrix_primitive_node_type!(Mat4, Quad, Quad, Mat4);
impl_matrix_primitive_node_type!(Mat2x3, Bi, Tri, Mat3x2);
impl_matrix_primitive_node_type!(Mat2x4, Bi, Quad, Mat4x2);
impl_matrix_primitive_node_type!(Mat3x2, Tri, Bi, Mat2x3);
impl_matrix_primitive_node_type!(Mat3x4, Tri, Quad, Mat4x3);
impl_matrix_primitive_node_type!(Mat4x2, Quad, Bi, Mat2x4);
impl_matrix_primitive_node_type!(Mat4x3, Quad, Tri, Mat3x4);

sg_node_impl!(
  Bool,
  ShaderValueSingleType::Sized(ShaderSizedValueType::Primitive(
    PrimitiveShaderValueType::u32()
  ))
);
impl ShaderSizedValueNodeType for Bool {
  fn sized_ty() -> ShaderSizedValueType {
    ShaderSizedValueType::Primitive(PrimitiveShaderValueType::u32())
  }
  fn to_value(&self) -> ShaderStructFieldInitValue {
    ShaderStructFieldInitValue::Primitive(self.to_primitive())
  }
}

impl PrimitiveShaderNodeType for Bool {
  fn primitive_ty() -> PrimitiveShaderValueType {
    PrimitiveShaderValueType::u32()
  }
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Scalar(ScalarValue::U32(self.0))
  }
}
impl Node<Bool> {
  pub fn into_bool(&self) -> Node<bool> {
    OperatorNode::Binary {
      left: self.handle(),
      right: val(0_u32).handle(),
      operator: BinaryOperator::NotEq,
    }
    .insert_api()
  }
}

impl Node<bool> {
  pub fn into_big_bool(&self) -> Node<Bool> {
    unsafe { self.select(val(1_u32), val(0_u32)).cast_type() }
  }
}

const fn assert_swizzle_component(component: u8, size: u8) {
  assert!(
    component < size,
    "the swizzle component is out of the vector size"
  );
}

fn swizzle_raw(source: ShaderNodeRawHandle, components: &[u8]) -> ShaderNodeRawHandle {
  let size = match components.len() {
    2 => VectorSize::Bi,
    3 => VectorSize::Tri,
    4 => VectorSize::Quad,
    _ => unreachable!("invalid swizzle size"),
  };
  let mut pattern = [0; 4];
  pattern[..components.len()].copy_from_slice(components);
  ShaderNodeExpr::Swizzle {
    source,
    size,
    pattern,
  }
  .insert_api_raw()
}

/// The component selection and swizzle by the component indices, the out of range index is a
/// compile error. The named ones (`.x()`, `.zyx()`, `.bgra()`, ...) are generated by
/// `impl_shader_swizzles` and implemented by these.
impl<V: ShaderVec> Node<V> {
  /// `v.component::<2>()` is `v.z`
  #[inline(always)]
  pub fn component<const I: u8>(&self) -> Node<V::Item> {
    const { assert_swizzle_component(I, V::SIZE) };
    unsafe { index_access_field(self.handle(), I as usize).into_node() }
  }

  /// `v.swizzle2::<1, 0>()` is `v.yx`
  #[inline(always)]
  pub fn swizzle2<const A: u8, const B: u8>(&self) -> Node<Vec2<V::Item>> {
    const { assert_swizzle_component(A, V::SIZE) };
    const { assert_swizzle_component(B, V::SIZE) };
    unsafe { swizzle_raw(self.handle(), &[A, B]).into_node() }
  }

  /// `v.swizzle3::<2, 1, 0>()` is `v.zyx`
  #[inline(always)]
  pub fn swizzle3<const A: u8, const B: u8, const C: u8>(&self) -> Node<Vec3<V::Item>> {
    const { assert_swizzle_component(A, V::SIZE) };
    const { assert_swizzle_component(B, V::SIZE) };
    const { assert_swizzle_component(C, V::SIZE) };
    unsafe { swizzle_raw(self.handle(), &[A, B, C]).into_node() }
  }

  /// `v.swizzle4::<3, 2, 1, 0>()` is `v.wzyx`
  #[inline(always)]
  pub fn swizzle4<const A: u8, const B: u8, const C: u8, const D: u8>(
    &self,
  ) -> Node<Vec4<V::Item>> {
    const { assert_swizzle_component(A, V::SIZE) };
    const { assert_swizzle_component(B, V::SIZE) };
    const { assert_swizzle_component(C, V::SIZE) };
    const { assert_swizzle_component(D, V::SIZE) };
    unsafe { swizzle_raw(self.handle(), &[A, B, C, D]).into_node() }
  }
}

impl_shader_swizzles!();

impl<T: ShaderScalarType> Node<T> {
  pub fn splat<V>(&self) -> Node<V>
  where
    V: ShaderVec<Item = T>,
  {
    let channel_count = match V::primitive_ty() {
      PrimitiveShaderValueType::Vector { size, .. } => size as usize,
      _ => unreachable!("shader vec must be vector type"),
    };
    ShaderNodeExpr::Compose {
      target: V::sized_ty(),
      parameters: vec![self.handle(); channel_count],
    }
    .insert_api()
  }
}

macro_rules! matrix_columns {
  ($Mat: ident, $Column: ident, { $($name: ident: $index: expr),+ }) => {
    impl<T> Node<$Mat<T>>
    where
      T: ShaderFloatType + Into<ScalarValue>,
    {
      $(
        #[inline(always)]
        pub fn $name(&self) -> Node<$Column<T>> {
          unsafe { index_access_field(self.handle(), $index).into_node() }
        }
      )+
    }
  };
}

matrix_columns!(Mat2, Vec2, { x: 0, y: 1 });
matrix_columns!(Mat3, Vec3, { x: 0, y: 1, z: 2 });
matrix_columns!(Mat4, Vec4, { x: 0, y: 1, z: 2, w: 3 });
matrix_columns!(Mat2x3, Vec3, { x: 0, y: 1 });
matrix_columns!(Mat2x4, Vec4, { x: 0, y: 1 });
matrix_columns!(Mat3x2, Vec2, { x: 0, y: 1, z: 2 });
matrix_columns!(Mat3x4, Vec4, { x: 0, y: 1, z: 2 });
matrix_columns!(Mat4x2, Vec2, { x: 0, y: 1, z: 2, w: 3 });
matrix_columns!(Mat4x3, Vec3, { x: 0, y: 1, z: 2, w: 3 });

fn convert_num<D: ShaderScalarType>(source: ShaderNodeRawHandle) -> ShaderNodeExpr {
  let convert_to = D::scalar_type();
  ShaderNodeExpr::Convert {
    source,
    convert_to,
    convert: Some(convert_to.byte_count() as u8),
  }
}

macro_rules! num_convert {
  ($src: ty, $dst: ty) => {
    paste::item! {
      impl Node<$src> {
        pub fn [< into_ $dst >](&self) -> Node<$dst> {
          convert_num::<$dst>(self.handle()).insert_api()
        }
      }
      impl Node<Vec2<$src>> {
        pub fn [< into_ $dst >](&self) -> Node<Vec2<$dst>> {
          convert_num::<$dst>(self.handle()).insert_api()
        }
      }
      impl Node<Vec3<$src>> {
        pub fn [< into_ $dst >](&self) -> Node<Vec3<$dst>> {
          convert_num::<$dst>(self.handle()).insert_api()
        }
      }
      impl Node<Vec4<$src>> {
        pub fn [< into_ $dst >](&self) -> Node<Vec4<$dst>> {
          convert_num::<$dst>(self.handle()).insert_api()
        }
      }
    }
  };
}

num_convert!(u32, f32);
num_convert!(f32, u32);
num_convert!(f32, i32);
num_convert!(i32, f32);
num_convert!(u32, i32);
num_convert!(i32, u32);
num_convert!(u32, bool);
num_convert!(bool, u32);
num_convert!(bool, i32);
num_convert!(i32, bool);
num_convert!(f32, bool);
num_convert!(bool, f32);

/// The 32 bit numeric scalar types, their scalars or vectors can be bitcast to each other.
///
/// see <https://www.w3.org/TR/WGSL/#bitcast-builtin>
pub trait ShaderBitcastScalarType: ShaderNumericScalarType {}
impl ShaderBitcastScalarType for f32 {}
impl ShaderBitcastScalarType for u32 {}
impl ShaderBitcastScalarType for i32 {}

impl<T> Node<T>
where
  T: ShaderScalarOrVec,
  T::Item: ShaderBitcastScalarType,
{
  /// Reinterpret the bits as `V`, which must be the same shape of scalar or vector, for example
  /// `bitcast::<u32>()` for f32, and `bitcast::<Vec3<u32>>()` for `Vec3<f32>`.
  pub fn bitcast<V>(self) -> Node<V>
  where
    V: ShaderScalarOrVec<Shape<T::Item> = T>,
    V::Item: ShaderBitcastScalarType,
  {
    ShaderNodeExpr::Convert {
      source: self.handle(),
      convert_to: V::Item::scalar_type(),
      convert: None,
    }
    .insert_api()
  }
}

macro_rules! impl_from {
  ( { $($field: tt: $constraint: ty),+ }, $type_merged:ty) => {
    impl From<( $(Node<$constraint>),+ )> for Node<$type_merged>
    {
      #[allow(non_snake_case)]
      fn from(($($field),+): ($(Node<$constraint>),+)) -> Self {
        $(let $field = $field.handle();)+
        ShaderNodeExpr::Compose {
          target: <$type_merged>::sized_ty(),
          parameters: vec![$($field),+],
        }
        .insert_api()
      }
    }
  }
}

macro_rules! compose_all_vec {
  ($t: ty) => {
    impl_from!({ A: $t, B: $t, C: $t, D: $t }, Vec4<$t>);
    impl_from!({ A: Vec2<$t>, B: $t, C: $t }, Vec4<$t>);
    impl_from!({ A: $t, B: Vec2<$t>, C: $t }, Vec4<$t>);
    impl_from!({ A: $t, B: $t, C: Vec2<$t> }, Vec4<$t>);
    impl_from!({ A: Vec3<$t>, B: $t }, Vec4<$t>);
    impl_from!({ A: $t, B: Vec3<$t> }, Vec4<$t>);
    impl_from!({ A: Vec2<$t>, B: Vec2<$t> }, Vec4<$t>);

    impl_from!({ A: $t, B: $t, C: $t }, Vec3<$t>);
    impl_from!({ A: $t, B: Vec2<$t> }, Vec3<$t>);
    impl_from!({ A: Vec2<$t>, B: $t }, Vec3<$t>);

    impl_from!({ A: $t, B: $t }, Vec2<$t>);

  }
}

macro_rules! compose_all_mat {
  ($t: ty) => {
    impl_from!({ A: Vec4<$t>, B: Vec4<$t>, C: Vec4<$t>, D:Vec4<$t> }, Mat4<$t>);
    impl_from!({ A: Vec3<$t>, B: Vec3<$t>, C: Vec3<$t> }, Mat3<$t>);
    impl_from!({ A: Vec2<$t>, B: Vec2<$t> }, Mat2<$t>);
    impl_from!({ A: Vec3<$t>, B: Vec3<$t> }, Mat2x3<$t>);
    impl_from!({ A: Vec4<$t>, B: Vec4<$t> }, Mat2x4<$t>);
    impl_from!({ A: Vec2<$t>, B: Vec2<$t>, C: Vec2<$t> }, Mat3x2<$t>);
    impl_from!({ A: Vec4<$t>, B: Vec4<$t>, C: Vec4<$t> }, Mat3x4<$t>);
    impl_from!({ A: Vec2<$t>, B: Vec2<$t>, C: Vec2<$t>, D: Vec2<$t> }, Mat4x2<$t>);
    impl_from!({ A: Vec3<$t>, B: Vec3<$t>, C: Vec3<$t>, D: Vec3<$t> }, Mat4x3<$t>);
  }
}

pub fn vec2_node<T>(x: impl Into<Node<Vec2<T>>>) -> Node<Vec2<T>> {
  x.into()
}
pub fn vec3_node<T>(x: impl Into<Node<Vec3<T>>>) -> Node<Vec3<T>> {
  x.into()
}
pub fn vec4_node<T>(x: impl Into<Node<Vec4<T>>>) -> Node<Vec4<T>> {
  x.into()
}
pub fn mat2_node<T>(x: impl Into<Node<Mat2<T>>>) -> Node<Mat2<T>> {
  x.into()
}
pub fn mat3_node<T>(x: impl Into<Node<Mat3<T>>>) -> Node<Mat3<T>> {
  x.into()
}
pub fn mat4_node<T>(x: impl Into<Node<Mat4<T>>>) -> Node<Mat4<T>> {
  x.into()
}

compose_all_vec!(f32);
compose_all_vec!(u32);
compose_all_vec!(i32);
compose_all_vec!(bool);
compose_all_mat!(f32);

impl Node<Mat4<f32>> {
  pub fn shrink_to_3(self) -> Node<Mat3<f32>> {
    let c1 = self.x();
    let c2 = self.y();
    let c3 = self.z();

    (c1.xyz(), c2.xyz(), c3.xyz()).into()
  }
}

impl Node<Mat4x3<f32>> {
  pub fn expand_to_4(self) -> Node<Mat4<f32>> {
    let c1 = self.x();
    let c2 = self.y();
    let c3 = self.z();
    let c4 = self.w();

    (
      (c1, val(0.)).into(),
      (c2, val(0.)).into(),
      (c3, val(0.)).into(),
      (c4, val(1.)).into(),
    )
      .into()
  }
}

impl Node<Mat4<f32>> {
  pub fn shrink_to_2(self) -> Node<Mat2<f32>> {
    let c1 = self.x();
    let c2 = self.y();

    (c1.xy(), c2.xy()).into()
  }
}

impl Node<Mat3<f32>> {
  pub fn shrink_to_2(self) -> Node<Mat2<f32>> {
    let c1 = self.x();
    let c2 = self.y();

    (c1.xy(), c2.xy()).into()
  }
}
