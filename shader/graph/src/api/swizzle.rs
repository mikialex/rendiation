use crate::*;

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
