use crate::*;

fn swizzle_node<I: ShaderGraphNodeType, T: ShaderGraphNodeType>(
  n: &Node<I>,
  ty: &'static str,
) -> Node<T> {
  let source = n.handle();
  ShaderGraphNodeExpr::Swizzle { ty, source }.insert_graph()
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

swizzle!(Vec4<f32>, Vec3<f32>, xyz);
swizzle!(Vec4<f32>, Vec2<f32>, xy);
swizzle!(Vec4<f32>, f32, x);
swizzle!(Vec4<u32>, u32, x);
swizzle!(Vec4<f32>, f32, y);
swizzle!(Vec4<f32>, f32, z);
swizzle!(Vec4<f32>, f32, w);

swizzle!(Vec3<f32>, f32, x);
swizzle!(Vec3<f32>, f32, y);
swizzle!(Vec3<f32>, f32, z);
swizzle!(Vec3<f32>, Vec2<f32>, xy);

swizzle!(Vec2<f32>, f32, x);
swizzle!(Vec2<f32>, f32, y);
// todo impl rest swizzle by magic

impl Node<u32> {
  pub fn as_f32(self) -> Node<f32> {
    let a = self.handle();
    ShaderGraphNodeExpr::Compose {
      target: f32::PRIMITIVE_TYPE,
      parameters: vec![a],
    }
    .insert_graph()
  }
}

impl<A, B> From<(A, B)> for Node<Vec4<f32>>
where
  A: Into<Node<Vec3<f32>>>,
  B: Into<Node<f32>>,
{
  fn from((a, b): (A, B)) -> Self {
    let a = a.into().handle();
    let b = b.into().handle();
    ShaderGraphNodeExpr::Compose {
      target: Vec4::<f32>::PRIMITIVE_TYPE,
      parameters: vec![a, b],
    }
    .insert_graph()
  }
}

impl<A, B, C, D> From<(A, B, C, D)> for Node<Vec4<f32>>
where
  A: Into<Node<f32>>,
  B: Into<Node<f32>>,
  C: Into<Node<f32>>,
  D: Into<Node<f32>>,
{
  fn from((a, b, c, d): (A, B, C, D)) -> Self {
    let a = a.into().handle();
    let b = b.into().handle();
    let c = c.into().handle();
    let d = d.into().handle();
    ShaderGraphNodeExpr::Compose {
      target: Vec4::<f32>::PRIMITIVE_TYPE,
      parameters: vec![a, b, c, d],
    }
    .insert_graph()
  }
}

impl<A, B, C, D> From<(A, B, C, D)> for Node<Mat4<f32>>
where
  A: Into<Node<Vec4<f32>>>,
  B: Into<Node<Vec4<f32>>>,
  C: Into<Node<Vec4<f32>>>,
  D: Into<Node<Vec4<f32>>>,
{
  fn from((a, b, c, d): (A, B, C, D)) -> Self {
    let a = a.into().handle();
    let b = b.into().handle();
    let c = c.into().handle();
    let d = d.into().handle();
    ShaderGraphNodeExpr::Compose {
      target: Mat4::<f32>::PRIMITIVE_TYPE,
      parameters: vec![a, b, c, d],
    }
    .insert_graph()
  }
}

impl<A, B, C> From<(A, B, C)> for Node<Mat3<f32>>
where
  A: Into<Node<Vec3<f32>>>,
  B: Into<Node<Vec3<f32>>>,
  C: Into<Node<Vec3<f32>>>,
{
  fn from((a, b, c): (A, B, C)) -> Self {
    let a = a.into().handle();
    let b = b.into().handle();
    let c = c.into().handle();
    ShaderGraphNodeExpr::Compose {
      target: Mat3::<f32>::PRIMITIVE_TYPE,
      parameters: vec![a, b, c],
    }
    .insert_graph()
  }
}

impl From<Node<Mat4<f32>>> for Node<Mat3<f32>> {
  fn from(n: Node<Mat4<f32>>) -> Self {
    ShaderGraphNodeExpr::MatShrink {
      source: n.handle(),
      dimension: 3,
    }
    .insert_graph()
  }
}
