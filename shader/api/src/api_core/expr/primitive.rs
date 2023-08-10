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
// swizzle_all!(i32);
// swizzle_all!(bool);

macro_rules! swizzle_mat {
  ($t: ty) => {
    swizzle!(Mat4<$t>, Vec4<$t>, x);
    swizzle!(Mat4<$t>, Vec4<$t>, y);
    swizzle!(Mat4<$t>, Vec4<$t>, z);
    swizzle!(Mat4<$t>, Vec4<$t>, w);
  };
}

swizzle_mat!(f32);

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
    impl From<( $(Node<$constraint>),+ )> for Node<$type_merged>
    {
      #[allow(non_snake_case)]
      fn from(($($field),+): ($(Node<$constraint>),+)) -> Self {
        $(let $field = $field.handle();)+
        ShaderGraphNodeExpr::Compose {
          target: <$type_merged>::PRIMITIVE_TYPE,
          parameters: vec![$($field),+],
        }
        .insert_graph()
      }
    }
  }
}

macro_rules! compose_all {
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

    impl_from!({ A: Vec4<$t>, B: Vec4<$t>, C: Vec4<$t>, D:Vec4<$t> }, Mat4<$t>);
    impl_from!({ A: Vec3<$t>, B: Vec3<$t>, C: Vec3<$t> }, Mat3<$t>);
  }
}

compose_all!(f32);

impl From<Node<Mat4<f32>>> for Node<Mat3<f32>> {
  fn from(n: Node<Mat4<f32>>) -> Self {
    ShaderGraphNodeExpr::MatShrink {
      source: n.handle(),
      dimension: 3,
    }
    .insert_graph()
  }
}
