use crate::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum PrimitiveShaderValueType {
  Bool,
  Int32,
  Uint32,
  Float32,
  Vec2Bool,
  Vec3Bool,
  Vec4Bool,
  Vec2Float32,
  Vec3Float32,
  Vec4Float32,
  Vec2Uint32,
  Vec3Uint32,
  Vec4Uint32,
  Vec2Int32,
  Vec3Int32,
  Vec4Int32,
  Mat2Float32,
  Mat3Float32,
  Mat4Float32,
}

impl PrimitiveShaderValueType {
  pub fn vertex_out_could_interpolated(self) -> bool {
    matches!(
      self,
      PrimitiveShaderValueType::Float32
        | PrimitiveShaderValueType::Vec2Float32
        | PrimitiveShaderValueType::Vec3Float32
        | PrimitiveShaderValueType::Vec4Float32
    )
  }
}

#[derive(Clone, Copy)]
pub enum PrimitiveShaderValue {
  Bool(bool),
  Uint32(u32),
  Int32(i32),
  Float32(f32),
  Vec2Bool(Vec2<bool>),
  Vec3Bool(Vec3<bool>),
  Vec4Bool(Vec4<bool>),
  Vec2Float32(Vec2<f32>),
  Vec3Float32(Vec3<f32>),
  Vec4Float32(Vec4<f32>),
  Vec2Uint32(Vec2<u32>),
  Vec3Uint32(Vec3<u32>),
  Vec4Uint32(Vec4<u32>),
  Vec2Int32(Vec2<i32>),
  Vec3Int32(Vec3<i32>),
  Vec4Int32(Vec4<i32>),
  Mat2Float32(Mat2<f32>),
  Mat3Float32(Mat3<f32>),
  Mat4Float32(Mat4<f32>),
}

impl PrimitiveShaderValue {
  pub fn into_raw_node(self) -> ShaderNodeRawHandle {
    match self {
      PrimitiveShaderValue::Bool(v) => val(v).handle(),
      PrimitiveShaderValue::Uint32(v) => val(v).handle(),
      PrimitiveShaderValue::Int32(v) => val(v).handle(),
      PrimitiveShaderValue::Float32(v) => val(v).handle(),
      PrimitiveShaderValue::Vec2Bool(v) => val(v).handle(),
      PrimitiveShaderValue::Vec3Bool(v) => val(v).handle(),
      PrimitiveShaderValue::Vec4Bool(v) => val(v).handle(),
      PrimitiveShaderValue::Vec2Float32(v) => val(v).handle(),
      PrimitiveShaderValue::Vec3Float32(v) => val(v).handle(),
      PrimitiveShaderValue::Vec4Float32(v) => val(v).handle(),
      PrimitiveShaderValue::Vec2Uint32(v) => val(v).handle(),
      PrimitiveShaderValue::Vec3Uint32(v) => val(v).handle(),
      PrimitiveShaderValue::Vec4Uint32(v) => val(v).handle(),
      PrimitiveShaderValue::Vec2Int32(v) => val(v).handle(),
      PrimitiveShaderValue::Vec3Int32(v) => val(v).handle(),
      PrimitiveShaderValue::Vec4Int32(v) => val(v).handle(),
      PrimitiveShaderValue::Mat2Float32(v) => val(v).handle(),
      PrimitiveShaderValue::Mat3Float32(v) => val(v).handle(),
      PrimitiveShaderValue::Mat4Float32(v) => val(v).handle(),
    }
  }
}

impl From<PrimitiveShaderValue> for PrimitiveShaderValueType {
  fn from(v: PrimitiveShaderValue) -> Self {
    match v {
      PrimitiveShaderValue::Bool(_) => PrimitiveShaderValueType::Bool,
      PrimitiveShaderValue::Int32(_) => PrimitiveShaderValueType::Int32,
      PrimitiveShaderValue::Uint32(_) => PrimitiveShaderValueType::Uint32,
      PrimitiveShaderValue::Float32(_) => PrimitiveShaderValueType::Float32,
      PrimitiveShaderValue::Vec2Bool(_) => PrimitiveShaderValueType::Vec2Bool,
      PrimitiveShaderValue::Vec3Bool(_) => PrimitiveShaderValueType::Vec3Bool,
      PrimitiveShaderValue::Vec4Bool(_) => PrimitiveShaderValueType::Vec4Bool,
      PrimitiveShaderValue::Vec2Float32(_) => PrimitiveShaderValueType::Vec2Float32,
      PrimitiveShaderValue::Vec3Float32(_) => PrimitiveShaderValueType::Vec3Float32,
      PrimitiveShaderValue::Vec4Float32(_) => PrimitiveShaderValueType::Vec4Float32,
      PrimitiveShaderValue::Mat2Float32(_) => PrimitiveShaderValueType::Mat2Float32,
      PrimitiveShaderValue::Mat3Float32(_) => PrimitiveShaderValueType::Mat3Float32,
      PrimitiveShaderValue::Mat4Float32(_) => PrimitiveShaderValueType::Mat4Float32,
      PrimitiveShaderValue::Vec2Uint32(_) => PrimitiveShaderValueType::Vec2Uint32,
      PrimitiveShaderValue::Vec3Uint32(_) => PrimitiveShaderValueType::Vec3Uint32,
      PrimitiveShaderValue::Vec4Uint32(_) => PrimitiveShaderValueType::Vec4Uint32,
      PrimitiveShaderValue::Vec2Int32(_) => PrimitiveShaderValueType::Vec2Int32,
      PrimitiveShaderValue::Vec3Int32(_) => PrimitiveShaderValueType::Vec3Int32,
      PrimitiveShaderValue::Vec4Int32(_) => PrimitiveShaderValueType::Vec4Int32,
    }
  }
}

// Impl Notes:
//
// impl<T: PrimitiveShaderNodeType> ShaderNodeType for T {
//   const TYPE: ShaderValueSingleType =
//     ShaderValueSingleType::Sized(ShaderSizedValueType::Primitive(T::PRIMITIVE_TYPE));
// }
// impl<T: PrimitiveShaderNodeType> ShaderSizedValueNodeType for T {
//   const TYPE: ShaderSizedValueType =
//     ShaderSizedValueType::Primitive(T::PRIMITIVE_TYPE);
// }
//
// We can not use above auto impl but the macro because rust not support trait associate const
// specialization

/// Impl note: why we not use the follow code instead of macro?
macro_rules! primitive_ty {
  ($ty: ty, $primitive_ty_value: expr, $to_primitive: expr) => {
    sg_node_impl!(
      $ty,
      ShaderValueSingleType::Sized(ShaderSizedValueType::Primitive($primitive_ty_value))
    );

    impl ShaderSizedValueNodeType for $ty {
      fn sized_ty() -> ShaderSizedValueType {
        ShaderSizedValueType::Primitive($primitive_ty_value)
      }
      fn to_value(&self) -> ShaderStructFieldInitValue {
        ShaderStructFieldInitValue::Primitive(self.to_primitive())
      }
    }

    impl PrimitiveShaderNodeType for $ty {
      const PRIMITIVE_TYPE: PrimitiveShaderValueType = $primitive_ty_value;
      type Shape<T> = T;
      fn to_primitive(&self) -> PrimitiveShaderValue {
        $to_primitive(*self)
      }
    }
  };
  ($ty: ty, $primitive_ty_value: expr, $to_primitive: expr, $shape: tt) => {
    sg_node_impl!(
      $ty,
      ShaderValueSingleType::Sized(ShaderSizedValueType::Primitive($primitive_ty_value))
    );

    impl ShaderSizedValueNodeType for $ty {
      fn sized_ty() -> ShaderSizedValueType {
        ShaderSizedValueType::Primitive($primitive_ty_value)
      }
      fn to_value(&self) -> ShaderStructFieldInitValue {
        ShaderStructFieldInitValue::Primitive(self.to_primitive())
      }
    }

    impl PrimitiveShaderNodeType for $ty {
      const PRIMITIVE_TYPE: PrimitiveShaderValueType = $primitive_ty_value;
      type Shape<T> = $shape<T>;
      fn to_primitive(&self) -> PrimitiveShaderValue {
        $to_primitive(*self)
      }
    }
  };
}

// we group them together just to skip rustfmt entirely
#[rustfmt::skip]
mod impls {
  use crate::*;
  primitive_ty!(bool, PrimitiveShaderValueType::Bool,  PrimitiveShaderValue::Bool);
  primitive_ty!(u32, PrimitiveShaderValueType::Uint32,  PrimitiveShaderValue::Uint32);
  primitive_ty!(i32, PrimitiveShaderValueType::Int32,  PrimitiveShaderValue::Int32);
  primitive_ty!(f32, PrimitiveShaderValueType::Float32,  PrimitiveShaderValue::Float32);
  primitive_ty!(Vec2<bool>, PrimitiveShaderValueType::Vec2Bool,  PrimitiveShaderValue::Vec2Bool, Vec2);
  primitive_ty!(Vec3<bool>, PrimitiveShaderValueType::Vec3Bool,  PrimitiveShaderValue::Vec3Bool, Vec3);
  primitive_ty!(Vec4<bool>, PrimitiveShaderValueType::Vec4Bool,  PrimitiveShaderValue::Vec4Bool, Vec4);
  primitive_ty!(Vec2<f32>, PrimitiveShaderValueType::Vec2Float32,  PrimitiveShaderValue::Vec2Float32, Vec2);
  primitive_ty!(Vec3<f32>, PrimitiveShaderValueType::Vec3Float32,  PrimitiveShaderValue::Vec3Float32, Vec3);
  primitive_ty!(Vec4<f32>, PrimitiveShaderValueType::Vec4Float32,  PrimitiveShaderValue::Vec4Float32, Vec4);
  primitive_ty!(Vec2<u32>, PrimitiveShaderValueType::Vec2Uint32,  PrimitiveShaderValue::Vec2Uint32, Vec2);
  primitive_ty!(Vec3<u32>, PrimitiveShaderValueType::Vec3Uint32,  PrimitiveShaderValue::Vec3Uint32, Vec3);
  primitive_ty!(Vec4<u32>, PrimitiveShaderValueType::Vec4Uint32,  PrimitiveShaderValue::Vec4Uint32, Vec4);
  primitive_ty!(Vec2<i32>, PrimitiveShaderValueType::Vec2Int32,  PrimitiveShaderValue::Vec2Int32, Vec2);
  primitive_ty!(Vec3<i32>, PrimitiveShaderValueType::Vec3Int32,  PrimitiveShaderValue::Vec3Int32, Vec3);
  primitive_ty!(Vec4<i32>, PrimitiveShaderValueType::Vec4Int32,  PrimitiveShaderValue::Vec4Int32, Vec4);
  primitive_ty!(Mat2<f32>, PrimitiveShaderValueType::Mat2Float32,  PrimitiveShaderValue::Mat2Float32, Mat2);
  primitive_ty!(Mat3<f32>, PrimitiveShaderValueType::Mat3Float32,  PrimitiveShaderValue::Mat3Float32, Mat3);
  primitive_ty!(Mat4<f32>, PrimitiveShaderValueType::Mat4Float32,  PrimitiveShaderValue::Mat4Float32, Mat4);
}

sg_node_impl!(
  Bool,
  ShaderValueSingleType::Sized(ShaderSizedValueType::Primitive(
    PrimitiveShaderValueType::Uint32
  ))
);
impl ShaderSizedValueNodeType for Bool {
  fn sized_ty() -> ShaderSizedValueType {
    ShaderSizedValueType::Primitive(PrimitiveShaderValueType::Uint32)
  }
  fn to_value(&self) -> ShaderStructFieldInitValue {
    ShaderStructFieldInitValue::Primitive(self.to_primitive())
  }
}

impl PrimitiveShaderNodeType for Bool {
  const PRIMITIVE_TYPE: PrimitiveShaderValueType = PrimitiveShaderValueType::Uint32;
  type Shape<T> = Bool;
  fn to_primitive(&self) -> PrimitiveShaderValue {
    PrimitiveShaderValue::Uint32(self.0)
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
