use crate::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PrimitiveShaderValueType {
  Bool,
  Int32,
  Uint32,
  Float32,
  Vec2Float32,
  Vec3Float32,
  Vec4Float32,
  Vec2Uint32,
  Vec3Uint32,
  Vec4Uint32,
  Mat2Float32,
  Mat3Float32,
  Mat4Float32,
}

pub enum PrimitiveScalarShaderType {
  Int32,
  Uint32,
  Float32,
}

#[derive(Clone, Copy)]
pub enum PrimitiveShaderValue {
  Bool(bool),
  Uint32(u32),
  Int32(i32),
  Float32(f32),
  Vec2Float32(Vec2<f32>),
  Vec3Float32(Vec3<f32>),
  Vec4Float32(Vec4<f32>),
  Vec2Uint32(Vec2<u32>),
  Vec3Uint32(Vec3<u32>),
  Vec4Uint32(Vec4<u32>),
  Mat2Float32(Mat2<f32>),
  Mat3Float32(Mat3<f32>),
  Mat4Float32(Mat4<f32>),
}

impl From<PrimitiveShaderValue> for PrimitiveShaderValueType {
  fn from(v: PrimitiveShaderValue) -> Self {
    match v {
      PrimitiveShaderValue::Int32(_) => PrimitiveShaderValueType::Int32,
      PrimitiveShaderValue::Uint32(_) => PrimitiveShaderValueType::Uint32,
      PrimitiveShaderValue::Float32(_) => PrimitiveShaderValueType::Float32,
      PrimitiveShaderValue::Vec2Float32(_) => PrimitiveShaderValueType::Vec2Float32,
      PrimitiveShaderValue::Vec3Float32(_) => PrimitiveShaderValueType::Vec3Float32,
      PrimitiveShaderValue::Vec4Float32(_) => PrimitiveShaderValueType::Vec4Float32,
      PrimitiveShaderValue::Mat2Float32(_) => PrimitiveShaderValueType::Mat2Float32,
      PrimitiveShaderValue::Mat3Float32(_) => PrimitiveShaderValueType::Mat3Float32,
      PrimitiveShaderValue::Mat4Float32(_) => PrimitiveShaderValueType::Mat4Float32,
      PrimitiveShaderValue::Bool(_) => PrimitiveShaderValueType::Bool,
      PrimitiveShaderValue::Vec2Uint32(_) => PrimitiveShaderValueType::Vec2Uint32,
      PrimitiveShaderValue::Vec3Uint32(_) => PrimitiveShaderValueType::Vec3Uint32,
      PrimitiveShaderValue::Vec4Uint32(_) => PrimitiveShaderValueType::Vec4Uint32,
    }
  }
}

// Impl Notes:
//
// impl<T: PrimitiveShaderGraphNodeType> ShaderGraphNodeType for T {
//   const TYPE: ShaderValueSingleType =
//     ShaderValueSingleType::Sized(ShaderSizedValueType::Primitive(T::PRIMITIVE_TYPE));
// }
// impl<T: PrimitiveShaderGraphNodeType> ShaderSizedValueNodeType for T {
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
      const MEMBER_TYPE: ShaderSizedValueType =
        ShaderSizedValueType::Primitive($primitive_ty_value);
    }

    impl PrimitiveShaderGraphNodeType for $ty {
      const PRIMITIVE_TYPE: PrimitiveShaderValueType = $primitive_ty_value;
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
  primitive_ty!(Vec2<f32>, PrimitiveShaderValueType::Vec2Float32,  PrimitiveShaderValue::Vec2Float32);
  primitive_ty!(Vec3<f32>, PrimitiveShaderValueType::Vec3Float32,  PrimitiveShaderValue::Vec3Float32);
  primitive_ty!(Vec4<f32>, PrimitiveShaderValueType::Vec4Float32,  PrimitiveShaderValue::Vec4Float32);
  primitive_ty!(Vec2<u32>, PrimitiveShaderValueType::Vec2Uint32,  PrimitiveShaderValue::Vec2Uint32);
  primitive_ty!(Vec3<u32>, PrimitiveShaderValueType::Vec3Uint32,  PrimitiveShaderValue::Vec3Uint32);
  primitive_ty!(Vec4<u32>, PrimitiveShaderValueType::Vec4Uint32,  PrimitiveShaderValue::Vec4Uint32);
  primitive_ty!(Mat2<f32>, PrimitiveShaderValueType::Mat2Float32,  PrimitiveShaderValue::Mat2Float32);
  primitive_ty!(Mat3<f32>, PrimitiveShaderValueType::Mat3Float32,  PrimitiveShaderValue::Mat3Float32);
  primitive_ty!(Mat4<f32>, PrimitiveShaderValueType::Mat4Float32,  PrimitiveShaderValue::Mat4Float32);
}

fn swizzle_node<I: ShaderGraphNodeType, T: ShaderGraphNodeType>(
  n: &Node<I>,
  ty: &'static str,
) -> Node<T> {
  let source = n.handle();
  ShaderGraphNodeExpr::Swizzle { ty, source }.insert_graph()
}

impl<T> Node<T>
where
  T: ShaderGraphNodeType + Scalar,
{
  pub fn splat<V>(&self) -> Node<V>
  where
    V: Vector<T> + ShaderGraphNodeType + PrimitiveShaderGraphNodeType,
  {
    ShaderGraphNodeExpr::Compose {
      target: V::PRIMITIVE_TYPE,
      parameters: vec![self.handle()],
    }
    .insert_graph()
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

// we don't have impl<T> for Vec3<T> for ShaderGraphNode, so we have to do macro
macro_rules! swizzle_all {
  ($t: ty) => {
    swizzle!(Vec4<$t>, Vec3<$t>, xyz);
    swizzle!(Vec4<$t>, Vec2<$t>, xy);
    swizzle!(Vec4<$t>, $t, x);
    swizzle!(Vec4<$t>, $t, y);
    swizzle!(Vec4<$t>, $t, z);
    swizzle!(Vec4<$t>, $t, w);

    swizzle!(Vec3<$t>, Vec2<$t>, xy);
    swizzle!(Vec3<$t>, $t, x);
    swizzle!(Vec3<$t>, $t, y);
    swizzle!(Vec3<$t>, $t, z);

    swizzle!(Vec2<$t>, $t, x);
    swizzle!(Vec2<$t>, $t, y);
  };
}

swizzle_all!(f32);
swizzle_all!(u32);

macro_rules! num_cast {
  ($src: ty, $dst: ty) => {
    paste::item! {
      impl Node<$src> {
        pub fn [< into_ $dst >](&self) -> Node<$dst> {
          let a = self.handle();
          ShaderGraphNodeExpr::Compose {
            target: $dst::PRIMITIVE_TYPE,
            parameters: vec![a],
          }
          .insert_graph()
        }
      }
    }
  };
}

num_cast!(u32, f32);
num_cast!(f32, u32);
num_cast!(f32, i32);
num_cast!(i32, f32);
num_cast!(u32, i32);

macro_rules! impl_from {
  ( { $($field: tt: $constraint: ty),+ }, $type_merged:ty) => {
    impl< $($field),+ > From<( $($field),+ )> for Node<$type_merged>
    where $($field: Into<Node<$constraint>>),+
    {
      #[allow(non_snake_case)]
      fn from(($($field),+): ($($field),+)) -> Self {
        $(let $field = $field.into().handle();)+
        ShaderGraphNodeExpr::Compose {
          target: <$type_merged>::PRIMITIVE_TYPE,
          parameters: vec![$($field),+],
        }
        .insert_graph()
      }
    }
  }
}

impl_from!({ A: f32, B: f32, C: f32, D: f32 }, Vec4<f32>);
impl_from!({ A: Vec2<f32>, B: f32, C: f32 }, Vec4<f32>);
impl_from!({ A: Vec3<f32>, B: f32 }, Vec4<f32>);

impl_from!({ A: f32, B: f32, C: f32 }, Vec3<f32>);

impl_from!({ A: f32, B: f32 }, Vec2<f32>);

impl_from!({ A: Vec4<f32>, B: Vec4<f32>, C: Vec4<f32>, D:Vec4<f32> }, Mat4<f32>);
impl_from!({ A: Vec3<f32>, B: Vec3<f32>, C: Vec3<f32> }, Mat3<f32>);

impl From<Node<Mat4<f32>>> for Node<Mat3<f32>> {
  fn from(n: Node<Mat4<f32>>) -> Self {
    ShaderGraphNodeExpr::MatShrink {
      source: n.handle(),
      dimension: 3,
    }
    .insert_graph()
  }
}
