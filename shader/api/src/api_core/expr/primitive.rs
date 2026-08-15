use crate::*;

#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub enum ScalarType {
  F32,
  U32,
  I32,
  Bool,
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

pub trait ScalarTypeOf {
  fn scalar_type() -> ScalarType;
}
impl ScalarTypeOf for bool {
  fn scalar_type() -> ScalarType {
    ScalarType::Bool
  }
}
impl ScalarTypeOf for u32 {
  fn scalar_type() -> ScalarType {
    ScalarType::U32
  }
}
impl ScalarTypeOf for i32 {
  fn scalar_type() -> ScalarType {
    ScalarType::I32
  }
}
impl ScalarTypeOf for f32 {
  fn scalar_type() -> ScalarType {
    ScalarType::F32
  }
}

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
    impl<T: ScalarTypeOf + Copy + Into<ScalarValue>> From<$ty> for PrimitiveShaderValue {
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
    impl<T: ScalarTypeOf + Copy + Into<ScalarValue>> From<$ty> for PrimitiveShaderValue {
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

// scalars are concrete types so they can not be grouped into one generic impl,
// vec and mat use generic impl over T: ScalarTypeOf to cover all supported scalar types.
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
      type Shape<X> = X;
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
      T: ScalarTypeOf + Copy + Into<ScalarValue> + Default + 'static,
    {
      fn single_ty() -> ShaderValueSingleType {
        ShaderValueSingleType::Sized(ShaderSizedValueType::Primitive(
          PrimitiveShaderValueType::vector(VectorSize::$size, T::scalar_type()),
        ))
      }
    }
    impl<T> ShaderNodeType for $ty<T>
    where
      T: ScalarTypeOf + Copy + Into<ScalarValue> + Default + 'static,
    {
      fn ty() -> ShaderValueType {
        ShaderValueType::Single(Self::single_ty())
      }
    }
    impl<T> ShaderSizedValueNodeType for $ty<T>
    where
      T: ScalarTypeOf + Copy + Into<ScalarValue> + Default + 'static,
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
      T: ScalarTypeOf + Copy + Into<ScalarValue> + Default + 'static,
    {
      fn primitive_ty() -> PrimitiveShaderValueType {
        PrimitiveShaderValueType::vector(VectorSize::$size, T::scalar_type())
      }
      type Shape<X> = $ty<X>;
      fn to_primitive(&self) -> PrimitiveShaderValue {
        PrimitiveShaderValue::from(*self)
      }
    }
  };
}

macro_rules! impl_matrix_primitive_node_type {
  ($ty: ident, $columns: ident, $rows: ident) => {
    impl<T> ShaderNodeSingleType for $ty<T>
    where
      T: ScalarTypeOf + Copy + Into<ScalarValue> + Default + 'static,
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
      T: ScalarTypeOf + Copy + Into<ScalarValue> + Default + 'static,
    {
      fn ty() -> ShaderValueType {
        ShaderValueType::Single(Self::single_ty())
      }
    }
    impl<T> ShaderSizedValueNodeType for $ty<T>
    where
      T: ScalarTypeOf + Copy + Into<ScalarValue> + Default + 'static,
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
      T: ScalarTypeOf + Copy + Into<ScalarValue> + Default + 'static,
    {
      fn primitive_ty() -> PrimitiveShaderValueType {
        PrimitiveShaderValueType::Matrix {
          columns: VectorSize::$columns,
          rows: VectorSize::$rows,
          scalar: T::scalar_type(),
        }
      }
      type Shape<X> = $ty<X>;
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
impl_matrix_primitive_node_type!(Mat2, Bi, Bi);
impl_matrix_primitive_node_type!(Mat3, Tri, Tri);
impl_matrix_primitive_node_type!(Mat4, Quad, Quad);
impl_matrix_primitive_node_type!(Mat4x3, Quad, Tri);

sg_node_impl!(
  Bool,
  ShaderValueSingleType::Sized(ShaderSizedValueType::Primitive(
    PrimitiveShaderValueType::Scalar(ScalarType::U32)
  ))
);
impl ShaderSizedValueNodeType for Bool {
  fn sized_ty() -> ShaderSizedValueType {
    ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Scalar(ScalarType::U32))
  }
  fn to_value(&self) -> ShaderStructFieldInitValue {
    ShaderStructFieldInitValue::Primitive(self.to_primitive())
  }
}

impl PrimitiveShaderNodeType for Bool {
  fn primitive_ty() -> PrimitiveShaderValueType {
    PrimitiveShaderValueType::Scalar(ScalarType::U32)
  }
  type Shape<T> = Bool;
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

fn swizzle_node<I: ShaderNodeType, T: ShaderNodeType>(n: &Node<I>, ty: &'static str) -> Node<T> {
  let source = n.handle();
  ShaderNodeExpr::Swizzle { ty, source }.insert_api()
}

impl<T> Node<T>
where
  T: ShaderNodeType + Scalar,
{
  pub fn splat<V>(&self) -> Node<V>
  where
    V: Vector<T> + ShaderSizedValueNodeType + PrimitiveShaderNodeType,
  {
    ShaderNodeExpr::Compose {
      target: V::sized_ty(),
      parameters: vec![self.handle(); V::channel_count()],
    }
    .insert_api()
  }
}

macro_rules! swizzle {
  ($IVec: ty, $OVec: ty, $Swi: ident) => {
    paste::item! {
      impl Node<$IVec> {
        pub fn [< $Swi >](&self) -> Node<$OVec> {
          swizzle_node::<_, _>(self, stringify!{$Swi})
        }
      }
    }
  };
}

macro_rules! swizzle_all {
  ($t: ty) => {
    swizzle!(Vec4<$t>, Vec3<$t>, xxy);
    swizzle!(Vec4<$t>, Vec3<$t>, xxz);
    swizzle!(Vec4<$t>, Vec3<$t>, xxx);
    swizzle!(Vec4<$t>, Vec3<$t>, xxw);
    swizzle!(Vec4<$t>, Vec3<$t>, xyx);
    swizzle!(Vec4<$t>, Vec3<$t>, xyz);
    swizzle!(Vec4<$t>, Vec3<$t>, xyy);
    swizzle!(Vec4<$t>, Vec3<$t>, xyw);
    swizzle!(Vec4<$t>, Vec3<$t>, xzx);
    swizzle!(Vec4<$t>, Vec3<$t>, xzy);
    swizzle!(Vec4<$t>, Vec3<$t>, xzz);
    swizzle!(Vec4<$t>, Vec3<$t>, xzw);
    swizzle!(Vec4<$t>, Vec3<$t>, xwx);
    swizzle!(Vec4<$t>, Vec3<$t>, xwy);
    swizzle!(Vec4<$t>, Vec3<$t>, xwz);
    swizzle!(Vec4<$t>, Vec3<$t>, xww);

    swizzle!(Vec4<$t>, Vec3<$t>, yxy);
    swizzle!(Vec4<$t>, Vec3<$t>, yxz);
    swizzle!(Vec4<$t>, Vec3<$t>, yxx);
    swizzle!(Vec4<$t>, Vec3<$t>, yxw);
    swizzle!(Vec4<$t>, Vec3<$t>, yyx);
    swizzle!(Vec4<$t>, Vec3<$t>, yyz);
    swizzle!(Vec4<$t>, Vec3<$t>, yyy);
    swizzle!(Vec4<$t>, Vec3<$t>, yyw);
    swizzle!(Vec4<$t>, Vec3<$t>, yzx);
    swizzle!(Vec4<$t>, Vec3<$t>, yzy);
    swizzle!(Vec4<$t>, Vec3<$t>, yzz);
    swizzle!(Vec4<$t>, Vec3<$t>, yzw);
    swizzle!(Vec4<$t>, Vec3<$t>, ywx);
    swizzle!(Vec4<$t>, Vec3<$t>, ywy);
    swizzle!(Vec4<$t>, Vec3<$t>, ywz);
    swizzle!(Vec4<$t>, Vec3<$t>, yww);

    swizzle!(Vec4<$t>, Vec3<$t>, zxy);
    swizzle!(Vec4<$t>, Vec3<$t>, zxz);
    swizzle!(Vec4<$t>, Vec3<$t>, zxx);
    swizzle!(Vec4<$t>, Vec3<$t>, zxw);
    swizzle!(Vec4<$t>, Vec3<$t>, zyx);
    swizzle!(Vec4<$t>, Vec3<$t>, zyz);
    swizzle!(Vec4<$t>, Vec3<$t>, zyy);
    swizzle!(Vec4<$t>, Vec3<$t>, zyw);
    swizzle!(Vec4<$t>, Vec3<$t>, zzx);
    swizzle!(Vec4<$t>, Vec3<$t>, zzy);
    swizzle!(Vec4<$t>, Vec3<$t>, zzz);
    swizzle!(Vec4<$t>, Vec3<$t>, zzw);
    swizzle!(Vec4<$t>, Vec3<$t>, zwx);
    swizzle!(Vec4<$t>, Vec3<$t>, zwy);
    swizzle!(Vec4<$t>, Vec3<$t>, zwz);
    swizzle!(Vec4<$t>, Vec3<$t>, zww);

    swizzle!(Vec4<$t>, Vec3<$t>, wxy);
    swizzle!(Vec4<$t>, Vec3<$t>, wxz);
    swizzle!(Vec4<$t>, Vec3<$t>, wxx);
    swizzle!(Vec4<$t>, Vec3<$t>, wxw);
    swizzle!(Vec4<$t>, Vec3<$t>, wyx);
    swizzle!(Vec4<$t>, Vec3<$t>, wyz);
    swizzle!(Vec4<$t>, Vec3<$t>, wyy);
    swizzle!(Vec4<$t>, Vec3<$t>, wyw);
    swizzle!(Vec4<$t>, Vec3<$t>, wzx);
    swizzle!(Vec4<$t>, Vec3<$t>, wzy);
    swizzle!(Vec4<$t>, Vec3<$t>, wzz);
    swizzle!(Vec4<$t>, Vec3<$t>, www);
    swizzle!(Vec4<$t>, Vec3<$t>, wwx);
    swizzle!(Vec4<$t>, Vec3<$t>, wwy);
    swizzle!(Vec4<$t>, Vec3<$t>, wwz);

    swizzle!(Vec4<$t>, Vec2<$t>, xy);
    swizzle!(Vec4<$t>, Vec2<$t>, xz);
    swizzle!(Vec4<$t>, Vec2<$t>, xx);
    swizzle!(Vec4<$t>, Vec2<$t>, xw);
    swizzle!(Vec4<$t>, Vec2<$t>, yx);
    swizzle!(Vec4<$t>, Vec2<$t>, yz);
    swizzle!(Vec4<$t>, Vec2<$t>, yy);
    swizzle!(Vec4<$t>, Vec2<$t>, yw);
    swizzle!(Vec4<$t>, Vec2<$t>, zx);
    swizzle!(Vec4<$t>, Vec2<$t>, zy);
    swizzle!(Vec4<$t>, Vec2<$t>, zz);
    swizzle!(Vec4<$t>, Vec2<$t>, zw);

    swizzle!(Vec4<$t>, $t, x);
    swizzle!(Vec4<$t>, $t, y);
    swizzle!(Vec4<$t>, $t, z);
    swizzle!(Vec4<$t>, $t, w);

    swizzle!(Vec3<$t>, Vec2<$t>, xy);
    swizzle!(Vec3<$t>, Vec2<$t>, xx);
    swizzle!(Vec3<$t>, Vec2<$t>, xz);
    swizzle!(Vec3<$t>, Vec2<$t>, yx);
    swizzle!(Vec3<$t>, Vec2<$t>, yy);
    swizzle!(Vec3<$t>, Vec2<$t>, yz);
    swizzle!(Vec3<$t>, Vec2<$t>, zx);
    swizzle!(Vec3<$t>, Vec2<$t>, zy);
    swizzle!(Vec3<$t>, Vec2<$t>, zz);
    swizzle!(Vec3<$t>, $t, x);
    swizzle!(Vec3<$t>, $t, y);
    swizzle!(Vec3<$t>, $t, z);

    swizzle!(Vec2<$t>, $t, x);
    swizzle!(Vec2<$t>, $t, y);
  };
}

swizzle_all!(f32);
swizzle_all!(u32);
swizzle_all!(i32);
// swizzle_all!(bool);

macro_rules! swizzle_mat {
  ($t: ty) => {
    swizzle!(Mat4<$t>, Vec4<$t>, x);
    swizzle!(Mat4<$t>, Vec4<$t>, y);
    swizzle!(Mat4<$t>, Vec4<$t>, z);
    swizzle!(Mat4<$t>, Vec4<$t>, w);

    swizzle!(Mat3<$t>, Vec3<$t>, x);
    swizzle!(Mat3<$t>, Vec3<$t>, y);
    swizzle!(Mat3<$t>, Vec3<$t>, z);

    swizzle!(Mat2<$t>, Vec2<$t>, x);
    swizzle!(Mat2<$t>, Vec2<$t>, y);
  };
}

swizzle_mat!(f32);

swizzle!(Mat4x3<f32>, Vec3<f32>, x);
swizzle!(Mat4x3<f32>, Vec3<f32>, y);
swizzle!(Mat4x3<f32>, Vec3<f32>, z);
swizzle!(Mat4x3<f32>, Vec3<f32>, w);

macro_rules! num_convert {
  ($src: ty, $dst: ty) => {
    paste::item! {
      impl Node<$src> {
        pub fn [< into_ $dst >](&self) -> Node<$dst> {
          let a = self.handle();
          ShaderNodeExpr::Convert {
            source: a,
            convert_to: $dst::KIND,
            convert: Some($dst::BYTE_WIDTH),
          }
          .insert_api()
        }
      }
      impl Node<Vec2<$src>> {
        pub fn [< into_ $dst >](&self) -> Node<Vec2<$dst>> {
          let a = self.handle();
          ShaderNodeExpr::Convert {
            source: a,
            convert_to: $dst::KIND,
            convert: Some($dst::BYTE_WIDTH),
          }
          .insert_api()
        }
      }
      impl Node<Vec3<$src>> {
        pub fn [< into_ $dst >](&self) -> Node<Vec3<$dst>> {
          let a = self.handle();
          ShaderNodeExpr::Convert {
            source: a,
            convert_to: $dst::KIND,
            convert: Some($dst::BYTE_WIDTH),
          }
          .insert_api()
        }
      }
      impl Node<Vec4<$src>> {
        pub fn [< into_ $dst >](&self) -> Node<Vec4<$dst>> {
          let a = self.handle();
          ShaderNodeExpr::Convert {
            source: a,
            convert_to: $dst::KIND,
            convert: Some($dst::BYTE_WIDTH),
          }
          .insert_api()
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

pub trait DeviceRawBitCast {
  type Value: ValueType;
}
impl DeviceRawBitCast for f32 {
  type Value = Self;
}
impl DeviceRawBitCast for u32 {
  type Value = Self;
}
impl DeviceRawBitCast for i32 {
  type Value = Self;
}
impl<T: ValueType> DeviceRawBitCast for Vec2<T> {
  type Value = T;
}
impl<T: ValueType> DeviceRawBitCast for Vec3<T> {
  type Value = T;
}
impl<T: ValueType> DeviceRawBitCast for Vec4<T> {
  type Value = T;
}

struct If<const B: bool>;
trait True {}
impl True for If<true> {}

impl<T: DeviceRawBitCast + PrimitiveShaderNodeType> Node<T> {
  // todo, impl vec bitcast
  #[allow(private_bounds)]
  pub fn bitcast<V>(self) -> Node<V>
  where
    V: DeviceRawBitCast + ValueType + PrimitiveShaderNodeType,
    If<{ std::mem::size_of::<T>() == std::mem::size_of::<V>() }>: True,
  {
    ShaderNodeExpr::Convert {
      source: self.handle(),
      convert_to: V::KIND,
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
